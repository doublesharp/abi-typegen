using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using System.Threading;
using System.Threading.Tasks;
using Nethereum.Hex.HexTypes;
using Nethereum.JsonRpc.Client;
using Nethereum.RPC.Eth.DTOs;

namespace AbiTypegen.Unity
{
    /// <summary>ABI-independent HTTP operations for generated calldata, filters and typed decoder delegates.</summary>
    public sealed class AbiTypegenRpcOperations
    {
        private readonly IClient client;
        /// <summary>Uses an existing Nethereum transport, including the Unity main-thread adapter.</summary>
        public AbiTypegenRpcOperations(IClient client) { this.client = client ?? throw new ArgumentNullException(nameof(client)); }

        /// <summary>Runs eth_call without submission and decodes its return bytes. RPC reverts propagate unchanged.</summary>
        public async Task<T> SimulateAsync<T>(TransactionInput transaction, Func<string, T> decode,
            BlockParameter block = null, CancellationToken cancellationToken = default)
        {
            ValidateTransaction(transaction);
            if (decode == null) throw new ArgumentNullException(nameof(decode));
            var data = await Rpc<string>("eth_call", cancellationToken, transaction, block ?? BlockParameter.CreateLatest());
            RequireHex(data, null, "call result");
            cancellationToken.ThrowIfCancellationRequested();
            return decode(data);
        }

        /// <summary>Estimates gas without submitting a transaction, retaining exact uint256 precision.</summary>
        public async Task<BigInteger> EstimateGasAsync(TransactionInput transaction, CancellationToken cancellationToken = default)
        {
            ValidateTransaction(transaction);
            var gas = await Rpc<HexBigInteger>("eth_estimateGas", cancellationToken, transaction);
            return Quantity(gas, "gas estimate");
        }

        /// <summary>Waits for a canonical confirmed successful receipt. Confirmed status-zero receipts raise an error.</summary>
        public async Task<TransactionReceipt> WaitForReceiptAsync(string transactionHash,
            AbiTypegenPollingOptions options = null, CancellationToken cancellationToken = default)
        {
            RequireHex(transactionHash, 64, nameof(transactionHash));
            var settings = (options ?? new AbiTypegenPollingOptions()).Snapshot();
            using (var timeout = new CancellationTokenSource(settings.Timeout))
            using (var linked = CancellationTokenSource.CreateLinkedTokenSource(timeout.Token, cancellationToken))
            {
                try
                {
                    while (true)
                    {
                        linked.Token.ThrowIfCancellationRequested();
                        var receipt = await Rpc<TransactionReceipt>("eth_getTransactionReceipt", linked.Token, transactionHash);
                        if (receipt != null)
                        {
                            ValidateReceipt(receipt, transactionHash);
                            var number = receipt.BlockNumber.Value;
                            var canonical = await ReadBlock(number, linked.Token);
                            if (canonical != null && EqualHash(canonical.BlockHash, receipt.BlockHash))
                            {
                                var head = Quantity(await Rpc<HexBigInteger>("eth_blockNumber", linked.Token), "head block");
                                if (head - number + 1 >= settings.Confirmations)
                                {
                                    // Read both receipt and block again: neither can be trusted across a moving head.
                                    var fresh = await Rpc<TransactionReceipt>("eth_getTransactionReceipt", linked.Token, transactionHash);
                                    if (fresh != null)
                                    {
                                        ValidateReceipt(fresh, transactionHash);
                                        var finalBlock = await ReadBlock(number, linked.Token);
                                        var finalHead = Quantity(await Rpc<HexBigInteger>("eth_blockNumber", linked.Token), "head block");
                                        if (fresh.BlockNumber.Value == number && EqualHash(fresh.BlockHash, receipt.BlockHash) &&
                                            finalBlock != null && EqualHash(finalBlock.BlockHash, fresh.BlockHash) &&
                                            finalHead - number + 1 >= settings.Confirmations)
                                        {
                                            if (fresh.Status.Value.IsZero)
                                                throw new InvalidOperationException("Confirmed transaction reverted: " + transactionHash);
                                            return fresh;
                                        }
                                    }
                                }
                            }
                        }
                        await Task.Delay(settings.PollInterval, linked.Token);
                    }
                }
                catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested && timeout.IsCancellationRequested)
                { throw new TimeoutException("Timed out waiting for a canonical transaction receipt."); }
            }
        }

        /// <summary>Fetches one bounded confirmed range and advances only after validation and typed decoding succeed.</summary>
        public async Task<IReadOnlyList<T>> PollEventsAsync<T>(NewFilterInput filter, AbiTypegenEventCursor cursor,
            Func<FilterLog, T> decode, AbiTypegenPollingOptions options = null, CancellationToken cancellationToken = default)
        {
            if (filter == null) throw new ArgumentNullException(nameof(filter));
            if (cursor == null) throw new ArgumentNullException(nameof(cursor));
            if (decode == null) throw new ArgumentNullException(nameof(decode));
            var settings = (options ?? new AbiTypegenPollingOptions()).Snapshot();
            var snapshot = SnapshotFilter(filter);
            using (var timeout = new CancellationTokenSource(settings.Timeout))
            using (var linked = CancellationTokenSource.CreateLinkedTokenSource(timeout.Token, cancellationToken))
            {
                try
                {
                    await cursor.Gate.WaitAsync(linked.Token);
                    try { return await PollRange(snapshot, cursor, decode, settings, linked.Token); }
                    finally { cursor.Gate.Release(); }
                }
                catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested && timeout.IsCancellationRequested)
                { throw new TimeoutException("Timed out polling contract events."); }
            }
        }

        /// <summary>Polls batches until cancellation, owner destruction, timeout or error. A handler must acknowledge each batch.</summary>
        public async Task WatchEventsAsync<T>(NewFilterInput filter, AbiTypegenEventCursor cursor, Func<FilterLog, T> decode,
            Func<IReadOnlyList<T>, CancellationToken, Task> onBatch, AbiTypegenPollingOptions options = null,
            CancellationToken cancellationToken = default)
        {
            if (filter == null) throw new ArgumentNullException(nameof(filter));
            if (cursor == null) throw new ArgumentNullException(nameof(cursor));
            if (decode == null) throw new ArgumentNullException(nameof(decode));
            if (onBatch == null) throw new ArgumentNullException(nameof(onBatch));
            var settings = (options ?? new AbiTypegenPollingOptions()).Snapshot();
            var snapshot = SnapshotFilter(filter);
            using (var timeout = new CancellationTokenSource(settings.Timeout))
            using (var linked = CancellationTokenSource.CreateLinkedTokenSource(timeout.Token, cancellationToken))
            {
                try
                {
                    // A watch owns its cursor so another consumer cannot race its acknowledgement.
                    await cursor.Gate.WaitAsync(linked.Token);
                    try
                    {
                        while (true)
                        {
                            var pending = new AbiTypegenEventCursor(cursor.NextBlock)
                            { CheckpointBlock = cursor.CheckpointBlock, CheckpointHash = cursor.CheckpointHash };
                            var batch = await PollRange(snapshot, pending, decode, settings, linked.Token);
                            if (batch.Count != 0) await AbiTypegenTask.WithCancellation(onBatch(batch, linked.Token), linked.Token);
                            linked.Token.ThrowIfCancellationRequested();
                            Commit(cursor, pending);
                            await Task.Delay(settings.PollInterval, linked.Token);
                        }
                    }
                    finally { cursor.Gate.Release(); }
                }
                catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested && timeout.IsCancellationRequested)
                { throw new TimeoutException("Timed out watching contract events."); }
            }
        }

        private async Task<IReadOnlyList<T>> PollRange<T>(NewFilterInput filter, AbiTypegenEventCursor cursor,
            Func<FilterLog, T> decode, AbiTypegenPollingOptions settings, CancellationToken token)
        {
            if (cursor.CheckpointBlock.HasValue)
            {
                var checkpoint = await ReadBlock(cursor.CheckpointBlock.Value, token);
                if (checkpoint == null || !EqualHash(checkpoint.BlockHash, cursor.CheckpointHash))
                    throw new InvalidOperationException("Event cursor checkpoint was reorganized; reconcile delivered events before creating a new cursor.");
            }
            var head = Quantity(await Rpc<HexBigInteger>("eth_blockNumber", token), "head block");
            var confirmed = head - settings.Confirmations + 1;
            if (confirmed < cursor.NextBlock) return Array.Empty<T>();
            var end = BigInteger.Min(confirmed, cursor.NextBlock + settings.MaxBlocksPerPoll - 1);
            RequireQuantity(end + 1, "next event cursor");
            var before = await ReadBlock(end, token);
            if (before == null) throw new InvalidOperationException("Confirmed range block is unavailable.");
            var request = SnapshotFilter(filter);
            request.FromBlock = new BlockParameter(new HexBigInteger(cursor.NextBlock));
            request.ToBlock = new BlockParameter(new HexBigInteger(end));
            var logs = await Rpc<FilterLog[]>("eth_getLogs", token, request);
            if (logs == null) throw new InvalidOperationException("Missing eth_getLogs result.");
            var canonical = new Dictionary<BigInteger, string> { [end] = before.BlockHash };
            var unique = new Dictionary<string, FilterLog>();
            foreach (var log in logs)
            {
                ValidateLog(log, cursor.NextBlock, end);
                if (!MatchesFilter(log, filter)) throw new InvalidOperationException("RPC returned a log outside the requested address/topic filter.");
                if (!canonical.TryGetValue(log.BlockNumber.Value, out var blockHash))
                {
                    var block = await ReadBlock(log.BlockNumber.Value, token);
                    if (block == null) throw new InvalidOperationException("Log block is unavailable.");
                    blockHash = block.BlockHash;
                    canonical.Add(log.BlockNumber.Value, blockHash);
                }
                if (!EqualHash(blockHash, log.BlockHash)) throw new InvalidOperationException("Log belongs to a noncanonical block.");
                var key = log.BlockNumber.Value + ":" + log.LogIndex.Value;
                if (unique.TryGetValue(key, out var prior))
                {
                    if (!SameLog(prior, log)) throw new InvalidOperationException("Conflicting duplicate log identity.");
                }
                else unique.Add(key, log);
            }
            var after = await ReadBlock(end, token);
            if (after == null || !EqualHash(before.BlockHash, after.BlockHash))
                throw new InvalidOperationException("Event range changed while being fetched; cursor was not advanced.");
            var decoded = unique.Values.OrderBy(log => log.BlockNumber.Value).ThenBy(log => log.LogIndex.Value).Select(decode).ToArray();
            token.ThrowIfCancellationRequested();
            cursor.NextBlock = end + 1;
            cursor.CheckpointBlock = end;
            cursor.CheckpointHash = after.BlockHash;
            return decoded;
        }

        private async Task<BlockWithTransactionHashes> ReadBlock(BigInteger number, CancellationToken token)
        {
            var block = await Rpc<BlockWithTransactionHashes>("eth_getBlockByNumber", token, new HexBigInteger(number).HexValue, false);
            if (block != null)
            {
                if (Quantity(block.Number, "block number") != number) throw new InvalidOperationException("RPC returned the wrong block number.");
                RequireHex(block.BlockHash, 64, "block hash");
            }
            return block;
        }

        private Task<T> Rpc<T>(string method, CancellationToken token, params object[] args)
        {
            token.ThrowIfCancellationRequested();
            return AbiTypegenTask.WithCancellation(client.SendRequestAsync<T>(method, null, args), token);
        }

        private static void ValidateTransaction(TransactionInput transaction)
        {
            if (transaction == null) throw new ArgumentNullException(nameof(transaction));
            AbiTypegenTransaction.Create(transaction.To, transaction.Data, transaction.From,
                transaction.Gas?.Value, transaction.Value?.Value, transaction.GasPrice?.Value);
            if (transaction.Nonce != null) RequireQuantity(transaction.Nonce.Value, "nonce");
            if (transaction.MaxFeePerGas != null) RequireQuantity(transaction.MaxFeePerGas.Value, "max fee per gas");
            if (transaction.MaxPriorityFeePerGas != null) RequireQuantity(transaction.MaxPriorityFeePerGas.Value, "max priority fee per gas");
        }

        private static void ValidateReceipt(TransactionReceipt receipt, string hash)
        {
            RequireHex(receipt.TransactionHash, 64, "receipt transaction hash");
            RequireHex(receipt.BlockHash, 64, "receipt block hash");
            if (!EqualHash(receipt.TransactionHash, hash)) throw new InvalidOperationException("Receipt transaction hash does not match the request.");
            Quantity(receipt.BlockNumber, "receipt block number");
            Quantity(receipt.TransactionIndex, "receipt transaction index");
            Quantity(receipt.GasUsed, "receipt gas used");
            if (Quantity(receipt.Status, "receipt status") > 1) throw new InvalidOperationException("Invalid receipt status.");
        }

        private static void ValidateLog(FilterLog log, BigInteger start, BigInteger end)
        {
            if (log == null) throw new InvalidOperationException("Null event log.");
            var number = Quantity(log.BlockNumber, "log block number");
            if (number < start || number > end || log.Removed == true) throw new InvalidOperationException("Log outside canonical requested range.");
            Quantity(log.TransactionIndex, "log transaction index");
            Quantity(log.LogIndex, "log index");
            RequireHex(log.BlockHash, 64, "log block hash");
            RequireHex(log.TransactionHash, 64, "log transaction hash");
            RequireHex(log.Address, 40, "log address");
            RequireHex(log.Data, null, "log data");
            if (log.Topics == null || log.Topics.Length > 4) throw new InvalidOperationException("Invalid event topics.");
            foreach (var topic in log.Topics) RequireHex(topic as string, 64, "log topic");
        }

        private static NewFilterInput SnapshotFilter(NewFilterInput filter)
        {
            if (filter.Address != null) foreach (var address in filter.Address) RequireHex(address, 40, "filter address");
            if (filter.Topics != null)
            {
                if (filter.Topics.Length > 4) throw new ArgumentException("At most four filter topics are supported.", nameof(filter));
                foreach (var topic in filter.Topics)
                {
                    if (topic == null) continue;
                    if (topic is string text) RequireHex(text, 64, "filter topic");
                    else if (topic is string[] alternatives) foreach (var alternative in alternatives) RequireHex(alternative, 64, "filter topic");
                    else throw new ArgumentException("Filter topics must be null, a hash or an array of hashes.", nameof(filter));
                }
            }
            return new NewFilterInput { Address = filter.Address?.ToArray(),
                Topics = filter.Topics?.Select(topic => topic is string[] alternatives ? (object)alternatives.ToArray() : topic).ToArray() };
        }

        private static bool MatchesFilter(FilterLog log, NewFilterInput filter)
        {
            if (filter.Address != null && filter.Address.Length != 0 && !filter.Address.Any(address => EqualHash(address, log.Address)))
                return false;
            if (filter.Topics == null) return true;
            for (var index = 0; index < filter.Topics.Length; index++)
            {
                var expected = filter.Topics[index];
                // Even wildcard positions are positional: an absent topic is not a wildcard match.
                if (index >= log.Topics.Length) return false;
                if (expected == null) continue;
                if (expected is string hash && !EqualHash(hash, (string)log.Topics[index])) return false;
                if (expected is string[] alternatives && !alternatives.Any(hash => EqualHash(hash, (string)log.Topics[index]))) return false;
            }
            return true;
        }

        private static bool SameLog(FilterLog left, FilterLog right) => EqualHash(left.TransactionHash, right.TransactionHash) &&
            left.TransactionIndex.Value == right.TransactionIndex.Value && EqualHash(left.Address, right.Address) &&
            EqualHash(left.Data, right.Data) && left.Topics.Cast<string>().SequenceEqual(right.Topics.Cast<string>(), StringComparer.OrdinalIgnoreCase);
        private static bool EqualHash(string left, string right) => string.Equals(left, right, StringComparison.OrdinalIgnoreCase);
        private static void Commit(AbiTypegenEventCursor destination, AbiTypegenEventCursor source)
        { destination.NextBlock = source.NextBlock; destination.CheckpointBlock = source.CheckpointBlock; destination.CheckpointHash = source.CheckpointHash; }
        private static BigInteger Quantity(HexBigInteger value, string name)
        {
            if (value == null) throw new InvalidOperationException("Missing " + name + ".");
            RequireQuantity(value.Value, name);
            return value.Value;
        }
        internal static void RequireQuantity(BigInteger value, string name)
        { if (value.Sign < 0 || value >= (BigInteger.One << 256)) throw new ArgumentOutOfRangeException(name, "Expected uint256."); }
        private static void RequireHex(string value, int? digits, string name)
        {
            if (value == null || !value.StartsWith("0x", StringComparison.OrdinalIgnoreCase) || (value.Length - 2) % 2 != 0 ||
                (digits.HasValue && value.Length - 2 != digits.Value) || value.Skip(2).Any(c => !Uri.IsHexDigit(c)))
                throw new InvalidOperationException("Malformed hexadecimal " + name + ".");
        }
    }
}
