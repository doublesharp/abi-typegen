using System;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace AbiTypegen.Unity
{
    /// <summary>
    /// Owns request lifetime for a GameObject. Destroying the object cancels callers immediately.
    /// For Nethereum APIs without native CancellationToken support this cancels the await, not the
    /// already-dispatched UnityWebRequest; its eventual fault/result is observed and discarded.
    /// </summary>
    public sealed class AbiTypegenRequestHost : MonoBehaviour
    {
        private readonly CancellationTokenSource destroyed = new CancellationTokenSource();
        private readonly CancellationToken destroyedToken;

        public AbiTypegenRequestHost()
        {
            destroyedToken = destroyed.Token;
        }

        public CancellationToken DestroyedToken => destroyedToken;

        public async Task<T> RunAsync<T>(Func<CancellationToken, Task<T>> operation, CancellationToken cancellationToken = default)
        {
            if (operation == null) throw new ArgumentNullException(nameof(operation));
            using (var linked = CancellationTokenSource.CreateLinkedTokenSource(destroyedToken, cancellationToken))
            {
                linked.Token.ThrowIfCancellationRequested();
                var task = operation(linked.Token);
                return await AbiTypegenTask.WithCancellation(task, linked.Token);
            }
        }

        public async Task RunAsync(Func<CancellationToken, Task> operation, CancellationToken cancellationToken = default)
        {
            if (operation == null) throw new ArgumentNullException(nameof(operation));
            using (var linked = CancellationTokenSource.CreateLinkedTokenSource(destroyedToken, cancellationToken))
            {
                linked.Token.ThrowIfCancellationRequested();
                var task = operation(linked.Token);
                await AbiTypegenTask.WithCancellation(task, linked.Token);
            }
        }

        private void OnDestroy()
        {
            if (!destroyed.IsCancellationRequested) destroyed.Cancel();
            destroyed.Dispose();
        }
    }
}
