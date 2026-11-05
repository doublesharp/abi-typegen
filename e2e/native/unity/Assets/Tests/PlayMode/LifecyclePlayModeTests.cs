using System;
using System.Collections;
using System.Numerics;
using System.Threading;
using System.Threading.Tasks;
using AbiTypegen.Unity;
using Nethereum.JsonRpc.Client;
using Nethereum.JsonRpc.Client.RpcMessages;
using Nethereum.RPC.Eth.DTOs;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;

namespace AbiTypegen.Unity.E2E.Tests
{
    public sealed class LifecyclePlayModeTests
    {
        [Test]
        public void AlreadyCanceledTokenWinsOverCompletedTasks()
        {
            using (var cancelled = new CancellationTokenSource())
            {
                cancelled.Cancel();
                Assert.That(AbiTypegenTask.WithCancellation(Task.FromResult(7), cancelled.Token).IsCanceled, Is.True);
                Assert.That(AbiTypegenTask.WithCancellation(Task.CompletedTask, cancelled.Token).IsCanceled, Is.True);
            }
        }

        [Test]
        public void TransactionInputRejectsMalformedHexAndOutOfRangeOptions()
        {
            const string address = "0x1111111111111111111111111111111111111111";
            Assert.Throws<ArgumentException>(() => AbiTypegenTransaction.Create("0x11", "0x1234", address));
            Assert.Throws<ArgumentException>(() => AbiTypegenTransaction.Create(address, "0x1234", "0x11"));
            Assert.Throws<ArgumentException>(() => AbiTypegenTransaction.Create(address, "0x123", address));
            Assert.Throws<ArgumentException>(() => AbiTypegenTransaction.Create(address, "0x12gg", address));
            Assert.Throws<ArgumentOutOfRangeException>(() => AbiTypegenTransaction.Create(
                address, "0x1234", address, value: BigInteger.One << 256));
            Assert.Throws<ArgumentOutOfRangeException>(() => AbiTypegenTransaction.Create(
                address, "0x1234", address, gasLimit: BigInteger.One << 256));
            Assert.Throws<ArgumentOutOfRangeException>(() => AbiTypegenTransaction.Create(
                address, "0x1234", address, gasPrice: BigInteger.One << 256));
            Assert.That(AbiTypegenTransaction.Create(address, "0x1234", address,
                value: BigInteger.One << 255).Value.Value, Is.EqualTo(BigInteger.One << 255));
        }

        [Test]
        public void PendingTaskEnumeratorTimesOut()
        {
            var pending = new TaskCompletionSource<int>();
            var iterator = AsyncEnumerator.Run(pending.Task, 0f);
            Assert.Throws<TimeoutException>(() => iterator.MoveNext());
        }

        [UnityTest]
        public IEnumerator ExternalCancellationStopsAwaitingPendingRequest()
        {
            var go = new GameObject("abi-typegen-cancel-test");
            var cts = new CancellationTokenSource();
            try
            {
                var host = go.AddComponent<AbiTypegenRequestHost>();
                var never = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
                var task = host.RunAsync(_ => never.Task, cts.Token);
                cts.Cancel();
                yield return WaitForCompletion(task);
                Assert.That(task.IsCanceled, Is.True);
            }
            finally
            {
                UnityEngine.Object.Destroy(go);
                cts.Dispose();
            }
        }

        [UnityTest]
        public IEnumerator DestroyingGameObjectCancelsPendingRequestWithoutUsingDestroyedObject()
        {
            var go = new GameObject("abi-typegen-destroy-test");
            try
            {
                var host = go.AddComponent<AbiTypegenRequestHost>();
                var never = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
                var task = host.RunAsync(_ => never.Task);
                UnityEngine.Object.Destroy(go);
                yield return WaitForCompletion(task);
                Assert.That(task.IsCanceled, Is.True);
                Assert.That(host.DestroyedToken.IsCancellationRequested, Is.True);
                Assert.That(host.RunAsync(_ => Task.FromResult(1)).IsCanceled, Is.True);
            }
            finally
            {
                if (go != null) UnityEngine.Object.Destroy(go);
            }
        }

        [UnityTest]
        public IEnumerator DestroyingOwnerStopsReceiptTrackingAndEventWatching()
        {
            var go = new GameObject("abi-typegen-polling-owner");
            try
            {
                var host = go.AddComponent<AbiTypegenRequestHost>();
                var operations = new AbiTypegenRpcOperations(new PendingRpcClient());
                var receipt = host.RunAsync(token => operations.WaitForReceiptAsync(
                    "0x1111111111111111111111111111111111111111111111111111111111111111", cancellationToken: token));
                var watch = host.RunAsync(token => operations.WatchEventsAsync(new NewFilterInput(),
                    new AbiTypegenEventCursor(0), log => log.Data,
                    (batch, batchToken) => Task.CompletedTask, cancellationToken: token));
                UnityEngine.Object.Destroy(go);
                yield return WaitForCompletion(Task.WhenAll(receipt, watch));
                Assert.That(receipt.IsCanceled, Is.True);
                Assert.That(watch.IsCanceled, Is.True);
            }
            finally
            {
                if (go != null) UnityEngine.Object.Destroy(go);
            }
        }

        private sealed class PendingRpcClient : IClient
        {
            public RequestInterceptor OverridingRequestInterceptor { get; set; }
            public Task<T> SendRequestAsync<T>(string method, string route = null, params object[] paramList) => new TaskCompletionSource<T>().Task;
            public Task SendRequestAsync(string method, string route = null, params object[] paramList) => throw new NotSupportedException();
            public T DecodeResult<T>(RpcResponseMessage response) => throw new NotSupportedException();
            public Task<RpcRequestResponseBatch> SendBatchRequestAsync(RpcRequestResponseBatch batch) => throw new NotSupportedException();
            public Task<T> SendRequestAsync<T>(RpcRequest request, string route = null) => throw new NotSupportedException();
            public Task SendRequestAsync(RpcRequest request, string route = null) => throw new NotSupportedException();
            public Task<RpcResponseMessage> SendAsync(RpcRequestMessage request, string route = null) => throw new NotSupportedException();
        }

        private static IEnumerator WaitForCompletion(Task task)
        {
            var started = Time.realtimeSinceStartup;
            while (!task.IsCompleted && Time.realtimeSinceStartup - started < 5f)
                yield return null;
            Assert.That(task.IsCompleted, Is.True, "Cancellation did not finish within five seconds.");
        }
    }
}
