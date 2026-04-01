// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

/// @title Decentralized exchange with complex types
/// @notice Exercises deep nesting, many functions, many events, and complex structs
contract Exchange {
    // ── Structs ────────────────────────────────────────────────────────
    struct Order {
        address maker;
        address taker;
        address tokenIn;
        address tokenOut;
        uint256 amountIn;
        uint256 amountOut;
        uint256 nonce;
        uint64 deadline;
        bytes32 salt;
        OrderType orderType;
        FeeConfig fees;
    }

    struct FeeConfig {
        uint16 makerFeeBps;
        uint16 takerFeeBps;
        address feeRecipient;
        bool discountEnabled;
    }

    struct Fill {
        bytes32 orderHash;
        uint256 amountFilled;
        uint256 amountRemaining;
        uint64 timestamp;
        address filler;
    }

    struct Pool {
        address token0;
        address token1;
        uint256 reserve0;
        uint256 reserve1;
        uint64 lastUpdate;
        uint16 feeBps;
        bool active;
    }

    struct Route {
        address[] path;
        uint256[] amounts;
        uint256 minOut;
        uint64 deadline;
    }

    struct BatchResult {
        bool[] successes;
        bytes[] results;
        uint256 totalGas;
    }

    enum OrderType { Limit, Market, StopLoss, TakeProfit }
    enum PoolStatus { Active, Paused, Deprecated }

    // ── State ──────────────────────────────────────────────────────────
    mapping(bytes32 => Order) public orders;
    mapping(bytes32 => Fill[]) public fills;
    mapping(bytes32 => Pool) public pools;
    mapping(address => mapping(address => uint256)) public balances;
    mapping(address => uint256) public nonces;

    uint256 public totalVolume;
    uint256 public totalFees;
    address public owner;
    bool public paused;

    // ── Events ─────────────────────────────────────────────────────────
    event OrderPlaced(bytes32 indexed orderHash, address indexed maker, OrderType orderType);
    event OrderFilled(bytes32 indexed orderHash, address indexed filler, uint256 amountFilled);
    event OrderCancelled(bytes32 indexed orderHash, address indexed maker);
    event PoolCreated(bytes32 indexed poolId, address token0, address token1, uint16 feeBps);
    event PoolUpdated(bytes32 indexed poolId, uint256 reserve0, uint256 reserve1);
    event Swap(address indexed user, address tokenIn, address tokenOut, uint256 amountIn, uint256 amountOut);
    event Deposit(address indexed user, address indexed token, uint256 amount);
    event Withdrawal(address indexed user, address indexed token, uint256 amount);
    event FeeCollected(address indexed recipient, address indexed token, uint256 amount);
    event Paused(address indexed by);
    event Unpaused(address indexed by);
    event OwnershipTransferred(address indexed from, address indexed to);

    // ── Errors ─────────────────────────────────────────────────────────
    error OrderExpired(bytes32 orderHash, uint64 deadline);
    error OrderAlreadyFilled(bytes32 orderHash);
    error InsufficientBalance(address token, uint256 available, uint256 required);
    error InvalidOrder(string reason);
    error PoolNotFound(bytes32 poolId);
    error PoolInactive(bytes32 poolId);
    error SlippageExceeded(uint256 expected, uint256 actual);
    error Unauthorized(address caller);
    error ContractPaused();
    error InvalidRoute(uint256 pathLength);
    error ZeroAmount();

    // ── Constructor ────────────────────────────────────────────────────
    constructor(address _owner) {
        owner = _owner;
    }

    // ── Order functions ────────────────────────────────────────────────
    /// @notice Place a new order
    /// @param order The order details
    /// @return orderHash The hash of the placed order
    function placeOrder(Order calldata order) external returns (bytes32 orderHash) {
        orderHash = keccak256(abi.encode(order, nonces[msg.sender]++));
        orders[orderHash] = order;
        emit OrderPlaced(orderHash, msg.sender, order.orderType);
    }

    /// @notice Fill an existing order
    /// @param orderHash Hash of the order to fill
    /// @param amount Amount to fill
    function fillOrder(bytes32 orderHash, uint256 amount) external {
        Order storage order = orders[orderHash];
        if (block.timestamp > order.deadline) revert OrderExpired(orderHash, order.deadline);
        if (amount == 0) revert ZeroAmount();
        fills[orderHash].push(Fill({
            orderHash: orderHash,
            amountFilled: amount,
            amountRemaining: order.amountIn - amount,
            timestamp: uint64(block.timestamp),
            filler: msg.sender
        }));
        emit OrderFilled(orderHash, msg.sender, amount);
    }

    /// @notice Cancel an order
    function cancelOrder(bytes32 orderHash) external {
        emit OrderCancelled(orderHash, msg.sender);
    }

    /// @notice Place multiple orders in one transaction
    function batchPlaceOrders(Order[] calldata orderList) external returns (bytes32[] memory hashes) {
        hashes = new bytes32[](orderList.length);
        for (uint256 i = 0; i < orderList.length; i++) {
            hashes[i] = keccak256(abi.encode(orderList[i], nonces[msg.sender]++));
            orders[hashes[i]] = orderList[i];
            emit OrderPlaced(hashes[i], msg.sender, orderList[i].orderType);
        }
    }

    // ── Pool functions ─────────────────────────────────────────────────
    /// @notice Create a new liquidity pool
    function createPool(address token0, address token1, uint16 feeBps) external returns (bytes32 poolId) {
        poolId = keccak256(abi.encodePacked(token0, token1));
        pools[poolId] = Pool({
            token0: token0,
            token1: token1,
            reserve0: 0,
            reserve1: 0,
            lastUpdate: uint64(block.timestamp),
            feeBps: feeBps,
            active: true
        });
        emit PoolCreated(poolId, token0, token1, feeBps);
    }

    /// @notice Add liquidity to a pool
    function addLiquidity(bytes32 poolId, uint256 amount0, uint256 amount1) external {
        Pool storage pool = pools[poolId];
        if (!pool.active) revert PoolInactive(poolId);
        pool.reserve0 += amount0;
        pool.reserve1 += amount1;
        pool.lastUpdate = uint64(block.timestamp);
        emit PoolUpdated(poolId, pool.reserve0, pool.reserve1);
    }

    // ── Swap functions ─────────────────────────────────────────────────
    /// @notice Swap tokens through a single pool
    function swap(address tokenIn, address tokenOut, uint256 amountIn, uint256 minOut) external returns (uint256 amountOut) {
        amountOut = amountIn; // simplified
        if (amountOut < minOut) revert SlippageExceeded(minOut, amountOut);
        totalVolume += amountIn;
        emit Swap(msg.sender, tokenIn, tokenOut, amountIn, amountOut);
    }

    /// @notice Swap through a multi-hop route
    function swapRoute(Route calldata route) external returns (uint256 finalAmount) {
        if (route.path.length < 2) revert InvalidRoute(route.path.length);
        finalAmount = route.amounts[0];
        for (uint256 i = 0; i < route.path.length - 1; i++) {
            emit Swap(msg.sender, route.path[i], route.path[i + 1], route.amounts[i], route.amounts[i + 1]);
            finalAmount = route.amounts[i + 1];
        }
    }

    // ── View functions ─────────────────────────────────────────────────
    /// @notice Get order details
    function getOrder(bytes32 orderHash) external view returns (Order memory) {
        return orders[orderHash];
    }

    /// @notice Get all fills for an order
    function getOrderFills(bytes32 orderHash) external view returns (Fill[] memory) {
        return fills[orderHash];
    }

    /// @notice Get pool info
    function getPool(bytes32 poolId) external view returns (Pool memory) {
        return pools[poolId];
    }

    /// @notice Quote a swap
    function quote(address tokenIn, address tokenOut, uint256 amountIn) external view returns (uint256 amountOut, uint256 fee) {
        amountOut = amountIn;
        fee = 0;
    }

    /// @notice Get multiple pool states
    function getPoolStates(bytes32[] calldata poolIds) external view returns (Pool[] memory poolStates) {
        poolStates = new Pool[](poolIds.length);
        for (uint256 i = 0; i < poolIds.length; i++) {
            poolStates[i] = pools[poolIds[i]];
        }
    }

    /// @notice Get user balances for multiple tokens
    function getUserBalances(address user, address[] calldata tokens) external view returns (uint256[] memory amounts) {
        amounts = new uint256[](tokens.length);
        for (uint256 i = 0; i < tokens.length; i++) {
            amounts[i] = balances[user][tokens[i]];
        }
    }

    // ── Admin functions ────────────────────────────────────────────────
    function pause() external {
        if (msg.sender != owner) revert Unauthorized(msg.sender);
        paused = true;
        emit Paused(msg.sender);
    }

    function unpause() external {
        if (msg.sender != owner) revert Unauthorized(msg.sender);
        paused = false;
        emit Unpaused(msg.sender);
    }

    function transferOwnership(address newOwner) external {
        if (msg.sender != owner) revert Unauthorized(msg.sender);
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    // ── Deposit/Withdraw ───────────────────────────────────────────────
    function deposit(address token, uint256 amount) external {
        if (amount == 0) revert ZeroAmount();
        balances[msg.sender][token] += amount;
        emit Deposit(msg.sender, token, amount);
    }

    function withdraw(address token, uint256 amount) external {
        if (balances[msg.sender][token] < amount) {
            revert InsufficientBalance(token, balances[msg.sender][token], amount);
        }
        balances[msg.sender][token] -= amount;
        emit Withdrawal(msg.sender, token, amount);
    }

    receive() external payable {
        emit Deposit(msg.sender, address(0), msg.value);
    }
}
