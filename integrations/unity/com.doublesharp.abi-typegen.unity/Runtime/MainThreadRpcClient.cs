using System;
using System.Threading;
using System.Threading.Tasks;
using Nethereum.JsonRpc.Client;
using Nethereum.JsonRpc.Client.RpcMessages;

namespace AbiTypegen.Unity
{
    /// <summary>Starts every UnityWebRequest RPC operation on the context that created the client.</summary>
    internal sealed class MainThreadRpcClient : IClient
    {
        private readonly IClient inner;
        private readonly SynchronizationContext mainThread;
        private readonly int mainThreadId;

        internal MainThreadRpcClient(IClient inner, SynchronizationContext mainThread, int mainThreadId)
        {
            this.inner = inner ?? throw new ArgumentNullException(nameof(inner));
            this.mainThread = mainThread ?? throw new ArgumentNullException(nameof(mainThread));
            this.mainThreadId = mainThreadId;
        }

        public RequestInterceptor OverridingRequestInterceptor
        {
            get => inner.OverridingRequestInterceptor;
            set => inner.OverridingRequestInterceptor = value;
        }

        public T DecodeResult<T>(RpcResponseMessage response) => inner.DecodeResult<T>(response);

        public Task<RpcRequestResponseBatch> SendBatchRequestAsync(RpcRequestResponseBatch batch) =>
            Dispatch(() => inner.SendBatchRequestAsync(batch));

        public Task<T> SendRequestAsync<T>(RpcRequest request, string route) =>
            Dispatch(() => inner.SendRequestAsync<T>(request, route));

        public Task SendRequestAsync(RpcRequest request, string route) =>
            Dispatch(() => inner.SendRequestAsync(request, route));

        public Task<T> SendRequestAsync<T>(string method, string route, params object[] paramList) =>
            Dispatch(() => inner.SendRequestAsync<T>(method, route, paramList));

        public Task SendRequestAsync(string method, string route, params object[] paramList) =>
            Dispatch(() => inner.SendRequestAsync(method, route, paramList));

        public Task<RpcResponseMessage> SendAsync(RpcRequestMessage request, string route) =>
            Dispatch(() => inner.SendAsync(request, route));

        private Task Dispatch(Func<Task> operation) =>
            Dispatch(async () =>
            {
                await operation().ConfigureAwait(false);
                return true;
            });

        private Task<T> Dispatch<T>(Func<Task<T>> operation)
        {
            if (Thread.CurrentThread.ManagedThreadId == mainThreadId)
                return operation();

            var completion = new TaskCompletionSource<T>(TaskCreationOptions.RunContinuationsAsynchronously);
            mainThread.Post(_ => { _ = CompleteAsync(operation, completion); }, null);
            return completion.Task;
        }

        private static async Task CompleteAsync<T>(Func<Task<T>> operation, TaskCompletionSource<T> completion)
        {
            try
            {
                completion.TrySetResult(await operation().ConfigureAwait(false));
            }
            catch (OperationCanceledException)
            {
                completion.TrySetCanceled();
            }
            catch (Exception error)
            {
                completion.TrySetException(error);
            }
        }
    }
}
