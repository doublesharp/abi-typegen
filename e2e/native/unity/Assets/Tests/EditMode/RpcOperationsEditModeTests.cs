using System;
using System.Collections.Generic;
using System.Numerics;
using System.Threading;
using System.Threading.Tasks;
using Nethereum.Hex.HexTypes;
using Nethereum.JsonRpc.Client;
using Nethereum.JsonRpc.Client.RpcMessages;
using Nethereum.RPC.Eth.DTOs;
using NUnit.Framework;

namespace AbiTypegen.Unity.E2E.Tests
{
    public sealed class RpcOperationsEditModeTests
    {
        [Test]
        public void AdapterExposesReusableTransactionAndEventOperations()
        {
            var operations = typeof(AbiTypegenTransaction).Assembly.GetType("AbiTypegen.Unity.AbiTypegenRpcOperations");
            Assert.That(operations, Is.Not.Null, "The reusable adapter must expose simulation, estimation, receipt tracking and event watching.");
        }

        private const string Hash = "0x1111111111111111111111111111111111111111111111111111111111111111";
        private const string OtherHash = "0x2222222222222222222222222222222222222222222222222222222222222222";
        private const string Address = "0x1111111111111111111111111111111111111111";

        [Test]
        public async Task SimulationAndGasEstimationRetainExactIntegers()
        {
            var large = (BigInteger.One << 200) + 7;
            var client = new ScriptedClient((method, args) => method == "eth_call" ? (object)"0x1234" : new HexBigInteger(large));
            var operations = new AbiTypegenRpcOperations(client);
            var transaction = AbiTypegenTransaction.Create(Address, "0x12", Address, value: large);
            Assert.That(await operations.SimulateAsync(transaction, data => data), Is.EqualTo("0x1234"));
            Assert.That(await operations.EstimateGasAsync(transaction), Is.EqualTo(large));
        }

        [Test]
        public async Task ReceiptWaitDiscardsOrphanThenChecksCanonicalConfirmations()
        {
            var reads = 0;
            var client = new ScriptedClient((method, args) =>
            {
                if (method == "eth_getTransactionReceipt") return Receipt(++reads == 1 ? OtherHash : Hash);
                if (method == "eth_blockNumber") return new HexBigInteger(11);
                return Block(10);
            });
            var result = await new AbiTypegenRpcOperations(client).WaitForReceiptAsync(Hash,
                new AbiTypegenPollingOptions { Confirmations = 2, PollInterval = TimeSpan.FromMilliseconds(1) });
            Assert.That(result.BlockHash, Is.EqualTo(Hash));
            Assert.That(reads, Is.EqualTo(3));
        }

        [Test]
        public void ReceiptWaitRejectsMalformedReceiptAndInvalidOptions()
        {
            var bad = Receipt(Hash);
            bad.Status = new HexBigInteger(2);
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) => bad));
            Assert.That(async () => await operations.WaitForReceiptAsync(Hash), Throws.TypeOf<InvalidOperationException>());
            Assert.That(async () => await operations.WaitForReceiptAsync(Hash,
                new AbiTypegenPollingOptions { Confirmations = 0 }), Throws.TypeOf<ArgumentOutOfRangeException>());
        }

        [Test]
        public void ReceiptWaitTimesOutAndCancellationStopsPendingRpc()
        {
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) => null));
            Assert.That(async () => await operations.WaitForReceiptAsync(Hash,
                new AbiTypegenPollingOptions { Timeout = TimeSpan.FromMilliseconds(10), PollInterval = TimeSpan.FromMilliseconds(1) }), Throws.TypeOf<TimeoutException>());
            using (var cancelled = new CancellationTokenSource())
            {
                cancelled.Cancel();
                Assert.That(async () => await operations.WaitForReceiptAsync(Hash, cancellationToken: cancelled.Token), Throws.InstanceOf<OperationCanceledException>());
            }
        }

        [Test]
        public async Task EventCursorSortsDeduplicatesAndRetainsBindingFilters()
        {
            var logs = new[] { Log(1), Log(0), Log(1) };
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
            {
                if (method == "eth_blockNumber") return new HexBigInteger(10);
                if (method == "eth_getLogs")
                {
                    var request = (NewFilterInput)args[0];
                    Assert.That(request.Address, Is.EqualTo(new[] { Address }));
                    Assert.That(request.Topics[0], Is.EqualTo(Hash));
                    Assert.That(request.FromBlock.BlockNumber.Value, Is.EqualTo(new BigInteger(10)));
                    return logs;
                }
                return Block(10);
            }));
            var cursor = new AbiTypegenEventCursor(10);
            var filter = new NewFilterInput { Address = new[] { Address }, Topics = new object[] { Hash } };
            var originalFrom = filter.FromBlock;
            var result = await operations.PollEventsAsync(filter, cursor, log => log.LogIndex.Value);
            Assert.That(result, Is.EqualTo(new[] { BigInteger.Zero, BigInteger.One }));
            Assert.That(cursor.NextBlock, Is.EqualTo(new BigInteger(11)));
            Assert.That(filter.FromBlock, Is.SameAs(originalFrom), "Caller-owned filters must not be mutated.");
        }

        [Test]
        public void FailedLogRangeDoesNotAdvanceCursorOrPartiallyDecode()
        {
            var decoded = 0;
            var bad = Log(1);
            bad.BlockHash = OtherHash;
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
                method == "eth_blockNumber" ? (object)new HexBigInteger(10) :
                method == "eth_getLogs" ? new[] { Log(0), bad } : (object)Block(10)));
            var cursor = new AbiTypegenEventCursor(10);
            Assert.That(async () => await operations.PollEventsAsync(
                new NewFilterInput(), cursor, log => ++decoded), Throws.TypeOf<InvalidOperationException>());
            Assert.That(cursor.NextBlock, Is.EqualTo(new BigInteger(10)));
            Assert.That(decoded, Is.Zero);
        }

        [Test]
        public void ProviderLogsOutsideAddressOrPositionalTopicFilterRejectTheWholeRange()
        {
            var filters = new[]
            {
                new NewFilterInput { Address = new[] { "0x2222222222222222222222222222222222222222" } },
                new NewFilterInput { Topics = new object[] { OtherHash } },
                new NewFilterInput { Topics = new object[] { null, new[] { Hash } } }
            };
            foreach (var filter in filters)
            {
                var decoded = 0;
                var cursor = new AbiTypegenEventCursor(10);
                Assert.That(async () => await EventOperations().PollEventsAsync(filter, cursor, log => ++decoded),
                    Throws.TypeOf<InvalidOperationException>());
                Assert.That(decoded, Is.Zero);
                Assert.That(cursor.NextBlock, Is.EqualTo(new BigInteger(10)));
            }
        }

        [Test]
        public async Task TopicAlternativesAndAddressesMatchCaseInsensitively()
        {
            var log = Log(0);
            log.Address = "0x" + new string('a', 40);
            var topic = "0x" + new string('b', 64);
            log.Topics = new object[] { topic };
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
                method == "eth_blockNumber" ? (object)new HexBigInteger(10) :
                method == "eth_getLogs" ? new[] { log } : (object)Block(10)));
            var filter = new NewFilterInput { Address = new[] { "0x" + new string('A', 40) },
                Topics = new object[] { new[] { OtherHash, "0x" + new string('B', 64) } } };
            var events = await operations.PollEventsAsync(filter, new AbiTypegenEventCursor(10), entry => entry.Data);
            Assert.That(events.Count, Is.EqualTo(1));
        }

        [Test]
        public void ExhaustedUint256CursorRejectsTheRangeBeforeFetchingOrDecodingLogs()
        {
            var max = (BigInteger.One << 256) - 1;
            var fetched = false;
            var decoded = 0;
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
            {
                if (method == "eth_blockNumber") return new HexBigInteger(max);
                if (method == "eth_getLogs") { fetched = true; return Array.Empty<FilterLog>(); }
                return new BlockWithTransactionHashes { Number = new HexBigInteger(max), BlockHash = Hash };
            }));
            var cursor = new AbiTypegenEventCursor(max);
            Assert.That(async () => await operations.PollEventsAsync(new NewFilterInput(), cursor, log => ++decoded),
                Throws.TypeOf<ArgumentOutOfRangeException>());
            Assert.That(fetched, Is.False);
            Assert.That(decoded, Is.Zero);
            Assert.That(cursor.NextBlock, Is.EqualTo(max));
        }

        [Test]
        public async Task ReorgBehindDeliveredCursorFailsExplicitly()
        {
            var changed = false;
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
                method == "eth_blockNumber" ? (object)new HexBigInteger(10) :
                method == "eth_getLogs" ? Array.Empty<FilterLog>() : (object)Block(10, changed ? OtherHash : Hash)));
            var cursor = new AbiTypegenEventCursor(10);
            await operations.PollEventsAsync(new NewFilterInput(), cursor, log => log.Data);
            changed = true;
            Assert.That(async () => await operations.PollEventsAsync(new NewFilterInput(), cursor, log => log.Data), Throws.TypeOf<InvalidOperationException>());
            Assert.That(cursor.NextBlock, Is.EqualTo(new BigInteger(11)));
        }

        [Test]
        public void ConfirmedFailedTransactionRaisesAnExplicitError()
        {
            var failed = Receipt(Hash);
            failed.Status = new HexBigInteger(0);
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
                method == "eth_getTransactionReceipt" ? (object)failed :
                method == "eth_blockNumber" ? new HexBigInteger(10) : (object)Block(10)));
            Assert.That(async () => await operations.WaitForReceiptAsync(Hash), Throws.TypeOf<InvalidOperationException>());
        }

        [Test]
        public void InvalidSimulationAndGasResultsFailAtTheRpcBoundary()
        {
            var transaction = AbiTypegenTransaction.Create(Address, "0x12", Address);
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
                method == "eth_call" ? (object)"0xgg" : new HexBigInteger(BigInteger.One << 256)));
            var decoded = false;
            Assert.That(async () => await operations.SimulateAsync(transaction, data => decoded = true), Throws.TypeOf<InvalidOperationException>());
            Assert.That(decoded, Is.False);
            Assert.That(async () => await operations.EstimateGasAsync(transaction), Throws.TypeOf<ArgumentOutOfRangeException>());
        }

        [Test]
        public void OutOfRangeNonceIsRejectedBeforeDispatch()
        {
            var dispatched = false;
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) => { dispatched = true; return new HexBigInteger(1); }));
            var transaction = AbiTypegenTransaction.Create(Address, "0x12", Address);
            transaction.Nonce = new HexBigInteger(BigInteger.One << 256);
            Assert.That(async () => await operations.EstimateGasAsync(transaction), Throws.TypeOf<ArgumentOutOfRangeException>());
            Assert.That(dispatched, Is.False);
        }

        [Test]
        public async Task CancellationStopsAnInFlightReceiptRequest()
        {
            var pending = new TaskCompletionSource<TransactionReceipt>();
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) => pending.Task));
            using (var cancelled = new CancellationTokenSource())
            {
                var wait = operations.WaitForReceiptAsync(Hash, cancellationToken: cancelled.Token);
                cancelled.Cancel();
                try { await wait; Assert.Fail("A cancelled wait must stop without waiting for the RPC response."); }
                catch (OperationCanceledException) { }
                Assert.That(pending.Task.IsCompleted, Is.False);
            }
        }

        [Test]
        public void SingleEventPollAlsoBoundsAPendingRpcRequest()
        {
            var pending = Task.Run(async () => { await Task.Delay(100); return new HexBigInteger(10); });
            var operations = new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
                method == "eth_blockNumber" ? (object)pending : method == "eth_getLogs" ? Array.Empty<FilterLog>() : (object)Block(10)));
            var cursor = new AbiTypegenEventCursor(10);
            Assert.That(async () => await operations.PollEventsAsync(new NewFilterInput(), cursor, log => log.Data,
                new AbiTypegenPollingOptions { Timeout = TimeSpan.FromMilliseconds(10) }), Throws.TypeOf<TimeoutException>());
            Assert.That(cursor.NextBlock, Is.EqualTo(new BigInteger(10)));
        }

        [Test]
        public void WatchHandlerFailureKeepsTheRangeUnacknowledged()
        {
            var operations = EventOperations();
            var cursor = new AbiTypegenEventCursor(10);
            Assert.That(async () => await operations.WatchEventsAsync(new NewFilterInput(), cursor, log => log.Data,
                (batch, token) => throw new ApplicationException("consumer rejected batch")), Throws.TypeOf<ApplicationException>());
            Assert.That(cursor.NextBlock, Is.EqualTo(new BigInteger(10)));
        }

        [Test]
        public async Task WatchCancellationAfterAcknowledgementNeverRedeliversTheBatch()
        {
            var operations = EventOperations();
            var cursor = new AbiTypegenEventCursor(10);
            var delivered = 0;
            using (var cancelled = new CancellationTokenSource())
            {
                var watching = operations.WatchEventsAsync(new NewFilterInput(), cursor, log => log.Data,
                    (batch, token) => { delivered += batch.Count; return Task.CompletedTask; },
                    new AbiTypegenPollingOptions { PollInterval = TimeSpan.FromMilliseconds(1) }, cancelled.Token);
                while (cursor.NextBlock == 10) await Task.Delay(1);
                cancelled.Cancel();
                try { await watching; Assert.Fail("The watcher must stop when cancelled."); } catch (OperationCanceledException) { }
                Assert.That(delivered, Is.EqualTo(1));
                Assert.That(cursor.NextBlock, Is.EqualTo(new BigInteger(11)));
            }
        }

        private static AbiTypegenRpcOperations EventOperations() => new AbiTypegenRpcOperations(new ScriptedClient((method, args) =>
            method == "eth_blockNumber" ? (object)new HexBigInteger(10) :
            method == "eth_getLogs" ? new[] { Log(0) } : (object)Block(10)));

        private static TransactionReceipt Receipt(string blockHash) => new TransactionReceipt
        {
            TransactionHash = Hash, BlockHash = blockHash, BlockNumber = new HexBigInteger(10),
            Status = new HexBigInteger(1), TransactionIndex = new HexBigInteger(0), GasUsed = new HexBigInteger(21000)
        };

        private static BlockWithTransactionHashes Block(int number, string hash = Hash) => new BlockWithTransactionHashes
        { Number = new HexBigInteger(number), BlockHash = hash };

        private static FilterLog Log(int index) => new FilterLog
        {
            Address = Address, Data = "0x", Topics = new object[] { Hash }, BlockNumber = new HexBigInteger(10),
            BlockHash = Hash, TransactionHash = Hash, TransactionIndex = new HexBigInteger(0), LogIndex = new HexBigInteger(index)
        };

        private sealed class ScriptedClient : IClient
        {
            private readonly Func<string, object[], object> respond;
            public ScriptedClient(Func<string, object[], object> respond) { this.respond = respond; }
            public RequestInterceptor OverridingRequestInterceptor { get; set; }
            public Task<T> SendRequestAsync<T>(string method, string route = null, params object[] paramList)
            {
                var result = respond(method, paramList);
                return result is Task<T> task ? task : Task.FromResult((T)result);
            }
            public Task SendRequestAsync(string method, string route = null, params object[] paramList) => throw new NotSupportedException();
            public T DecodeResult<T>(RpcResponseMessage response) => throw new NotSupportedException();
            public Task<RpcRequestResponseBatch> SendBatchRequestAsync(RpcRequestResponseBatch batch) => throw new NotSupportedException();
            public Task<T> SendRequestAsync<T>(RpcRequest request, string route = null) => throw new NotSupportedException();
            public Task SendRequestAsync(RpcRequest request, string route = null) => throw new NotSupportedException();
            public Task<RpcResponseMessage> SendAsync(RpcRequestMessage request, string route = null) => throw new NotSupportedException();
        }
    }
}
