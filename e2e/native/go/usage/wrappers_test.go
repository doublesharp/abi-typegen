package usage

import (
	"bytes"
	"context"
	"math/big"
	"testing"

	"github.com/doublesharp/abi-typegen/e2e/native/go/contracts"
	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
)

type bindingBackend struct {
	bind.ContractBackend
	result []byte
	logs   []types.Log
	call   ethereum.CallMsg
	query  ethereum.FilterQuery
}

func (b *bindingBackend) CallContract(_ context.Context, call ethereum.CallMsg, _ *big.Int) ([]byte, error) {
	b.call = call
	return b.result, nil
}

func (b *bindingBackend) CodeAt(context.Context, common.Address, *big.Int) ([]byte, error) {
	return []byte{1}, nil
}

func (b *bindingBackend) FilterLogs(_ context.Context, query ethereum.FilterQuery) ([]types.Log, error) {
	b.query = query
	return b.logs, nil
}

func TestGoBindingOfflineEncodingAndDecoding(t *testing.T) {
	token, err := contracts.NewTokenBinding(common.Address{}, nil)
	if err != nil {
		t.Fatal(err)
	}
	to := common.HexToAddress("0x1234")
	amount := big.NewInt(42)
	calldata, err := token.EncodeTransfer(contracts.TokenTransferParams{To: to, Amount: amount})
	if err != nil {
		t.Fatal(err)
	}
	want, err := token.ABI().Pack("transfer", to, amount)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(calldata, want) {
		t.Fatalf("transfer calldata mismatch: %x != %x", calldata, want)
	}
	returnData, err := token.ABI().Methods["transfer"].Outputs.Pack(true)
	if err != nil {
		t.Fatal(err)
	}
	result, err := token.DecodeTransferResult(returnData)
	if err != nil || !result.Success {
		t.Fatalf("transfer output: %#v, %v", result, err)
	}
	if _, err := token.DecodeTransferResult([]byte{1}); err == nil {
		t.Fatal("truncated return data accepted")
	}

	vault, err := contracts.NewVaultBinding(common.Address{}, nil)
	if err != nil {
		t.Fatal(err)
	}
	deposit, err := vault.EncodeDeposit(contracts.VaultDepositParams{Amount: amount, Recipient: to})
	if err != nil {
		t.Fatal(err)
	}
	deposit0, err := vault.EncodeDeposit0(contracts.VaultDeposit0Params{Amount: amount})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(deposit[:4], contracts.VaultDepositSelector[:]) || !bytes.Equal(deposit0[:4], contracts.VaultDeposit0Selector[:]) {
		t.Fatal("overloaded deposit selectors differ from canonical signatures")
	}
	position := contracts.VaultPosition{Shares: amount, DepositedAt: 9, Token: to}
	positionData, err := vault.ABI().Methods["getPosition"].Outputs.Pack(position)
	if err != nil {
		t.Fatal(err)
	}
	decodedPosition, err := vault.DecodeGetPositionResult(positionData)
	if err != nil || decodedPosition.Position.Shares.Cmp(amount) != 0 || decodedPosition.Position.Token != to {
		t.Fatalf("tuple output: %#v, %v", decodedPosition, err)
	}

	tuples, err := contracts.NewTupleCasesBinding(common.Address{}, nil)
	if err != nil {
		t.Fatal(err)
	}
	account := contracts.TupleCasesTupleAccountPosition{Account: to}
	encodedAccount, err := tuples.EncodeDeposit(contracts.TupleCasesDepositParams{Position: account})
	if err != nil {
		t.Fatal(err)
	}
	wantAccount, err := tuples.ABI().Pack("deposit", account)
	if err != nil || !bytes.Equal(encodedAccount, wantAccount) {
		t.Fatalf("tuple overload: %x, %x, %v", encodedAccount, wantAccount, err)
	}
	matrix := [][]*big.Int{{big.NewInt(1), big.NewInt(2)}, {big.NewInt(3)}}
	encodedMatrix, err := tuples.EncodeSetMatrix(contracts.TupleCasesSetMatrixParams{Matrix: matrix})
	if err != nil {
		t.Fatal(err)
	}
	wantMatrix, err := tuples.ABI().Pack("setMatrix", matrix)
	if err != nil || !bytes.Equal(encodedMatrix, wantMatrix) {
		t.Fatalf("nested array: %x, %x, %v", encodedMatrix, wantMatrix, err)
	}

	custom := token.ABI().Errors["InsufficientBalance"]
	args, err := custom.Inputs.Pack(to, big.NewInt(4), amount)
	if err != nil {
		t.Fatal(err)
	}
	revert := append(custom.ID[:4], args...)
	decodedError, err := token.DecodeError(revert)
	if err != nil {
		t.Fatal(err)
	}
	insufficient, ok := decodedError.(contracts.TokenInsufficientBalanceError)
	if !ok || insufficient.Account != to || insufficient.Required.Cmp(amount) != 0 {
		t.Fatalf("custom error: %#v", decodedError)
	}
	if _, err := token.DecodeError([]byte{1, 2, 3}); err == nil {
		t.Fatal("short revert data accepted")
	}
	if _, err := token.DecodeError(append(custom.ID[:4], 1)); err == nil {
		t.Fatal("malformed custom error arguments accepted")
	}
}

func TestGoBindingBackendCallAndEventFilter(t *testing.T) {
	address := common.HexToAddress("0x1111")
	owner := common.HexToAddress("0x2222")
	to := common.HexToAddress("0x3333")
	amount := big.NewInt(27)
	parsed, err := abi.JSON(stringsReader(contracts.TokenABI))
	if err != nil {
		t.Fatal(err)
	}
	result, err := parsed.Methods["balanceOf"].Outputs.Pack(amount)
	if err != nil {
		t.Fatal(err)
	}
	event := parsed.Events["Transfer"]
	data, err := event.Inputs.NonIndexed().Pack(amount)
	if err != nil {
		t.Fatal(err)
	}
	backend := &bindingBackend{result: result, logs: []types.Log{{Address: address, Topics: []common.Hash{event.ID, common.BytesToHash(owner.Bytes()), common.BytesToHash(to.Bytes())}, Data: data}}}
	contract, err := contracts.NewTokenBinding(address, backend)
	if err != nil {
		t.Fatal(err)
	}
	balance, err := contract.BalanceOf(nil, contracts.TokenBalanceOfParams{Address: owner})
	if err != nil || balance.Uint256.Cmp(amount) != 0 {
		t.Fatalf("typed read: %#v, %v", balance, err)
	}
	wantCall, err := parsed.Pack("balanceOf", owner)
	if err != nil || !bytes.Equal(backend.call.Data, wantCall) {
		t.Fatalf("backend calldata: %x, %x, %v", backend.call.Data, wantCall, err)
	}
	logs, err := contract.FilterTransfer(context.Background(), nil, nil, []any{owner})
	if err != nil || len(logs) != 1 || logs[0].From != owner || logs[0].To != to || logs[0].Amount.Cmp(amount) != 0 {
		t.Fatalf("decoded event logs: %#v, %v", logs, err)
	}
	if len(backend.query.Topics) != 2 || backend.query.Topics[0][0] != event.ID || backend.query.Topics[1][0] != common.BytesToHash(owner.Bytes()) {
		t.Fatalf("event filter topics: %#v", backend.query.Topics)
	}
	if _, err := contract.DecodeTransferEvent(types.Log{Topics: []common.Hash{common.Hash{1}}}); err == nil {
		t.Fatal("wrong event signature accepted")
	}
	if _, err := contract.DecodeTransferEvent(types.Log{Topics: backend.logs[0].Topics, Data: []byte{1}}); err == nil {
		t.Fatal("malformed event data accepted")
	}
}

func TestGoBindingIndexedHashesAndAnonymousEvent(t *testing.T) {
	contract, err := contracts.NewNativeCasesBinding(common.Address{}, nil)
	if err != nil {
		t.Fatal(err)
	}
	parsed := contract.ABI()
	refs := parsed.Events["IndexedReferences"]
	hashes := []common.Hash{common.HexToHash("0x01"), common.HexToHash("0x02"), common.HexToHash("0x03")}
	decoded, err := contract.DecodeIndexedReferencesEvent(types.Log{Topics: append([]common.Hash{refs.ID}, hashes...)})
	if err != nil || decoded.Label != hashes[0] || decoded.Payload != hashes[1] || decoded.Values != hashes[2] {
		t.Fatalf("indexed hashes: %#v, %v", decoded, err)
	}
	logged := parsed.Events["Logged"]
	data, err := logged.Inputs.Pack([]byte{7, 8})
	if err != nil {
		t.Fatal(err)
	}
	anonymous, err := contract.DecodeLoggedEvent(types.Log{Data: data})
	if err != nil || !bytes.Equal(anonymous.Data, []byte{7, 8}) {
		t.Fatalf("anonymous event: %#v, %v", anonymous, err)
	}
}

func TestGoBindingPayableValueAndNonpayableGuard(t *testing.T) {
	backend := &bindingBackend{}
	address := common.HexToAddress("0x4444")
	payable, err := contracts.NewVaultBinding(address, backend)
	if err != nil {
		t.Fatal(err)
	}
	opts := &bind.TransactOpts{
		Value:    big.NewInt(5),
		Nonce:    big.NewInt(0),
		GasPrice: big.NewInt(1),
		GasLimit: 100000,
		NoSend:   true,
		Signer:   func(_ common.Address, tx *types.Transaction) (*types.Transaction, error) { return tx, nil },
	}
	tx, err := payable.Deposit0(opts, contracts.VaultDeposit0Params{Amount: big.NewInt(1)})
	if err != nil || tx.Value().Cmp(big.NewInt(5)) != 0 {
		t.Fatalf("payable transaction value: %v, %v", tx, err)
	}
	if !bytes.Equal(tx.Data()[:4], contracts.VaultDeposit0Selector[:]) {
		t.Fatalf("payable transaction selector: %x", tx.Data()[:4])
	}
	nonpayable, err := contracts.NewTokenBinding(address, backend)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := nonpayable.Transfer(opts, contracts.TokenTransferParams{Amount: big.NewInt(1)}); err == nil {
		t.Fatal("nonpayable transaction accepted a value")
	}
	if _, err := nonpayable.Transfer(nil, contracts.TokenTransferParams{Amount: big.NewInt(1)}); err == nil {
		t.Fatal("nil transaction options accepted")
	}
}

func TestGoBindingTypedDeployment(t *testing.T) {
	backend := &bindingBackend{}
	bytecode := []byte{0x60, 0x00, 0x60, 0x00}
	opts := &bind.TransactOpts{
		Nonce: big.NewInt(3), GasPrice: big.NewInt(1), GasLimit: 100000, NoSend: true,
		Signer: func(_ common.Address, tx *types.Transaction) (*types.Transaction, error) { return tx, nil },
	}
	params := contracts.TokenConstructorParams{Name: "Name", Symbol: "SYM", Decimals: 8}
	address, tx, binding, err := contracts.DeployTokenBinding(opts, backend, bytecode, params)
	if err != nil || binding == nil || tx == nil || address == (common.Address{}) {
		t.Fatalf("typed deployment: %v, %v, %v, %v", address, tx, binding, err)
	}
	constructorData, err := binding.EncodeConstructor(params)
	if err != nil || !bytes.Equal(tx.Data(), append(bytecode, constructorData...)) {
		t.Fatalf("constructor calldata: %x, %x, %v", tx.Data(), constructorData, err)
	}
	valueOpts := *opts
	valueOpts.Value = big.NewInt(1)
	if _, _, _, err := contracts.DeployTokenBinding(&valueOpts, backend, bytecode, params); err == nil {
		t.Fatal("nonpayable constructor accepted value")
	}
}
