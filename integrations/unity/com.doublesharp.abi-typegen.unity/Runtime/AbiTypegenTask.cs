using System;
using System.Threading;
using System.Threading.Tasks;

namespace AbiTypegen.Unity
{
    /// <summary>Cancellation helpers for Nethereum Tasks that do not expose CancellationToken.</summary>
    public static class AbiTypegenTask
    {
        public static async Task<T> WithCancellation<T>(Task<T> task, CancellationToken cancellationToken)
        {
            if (task == null) throw new ArgumentNullException(nameof(task));
            if (!cancellationToken.CanBeCanceled) return await task.ConfigureAwait(false);
            if (cancellationToken.IsCancellationRequested)
            {
                ObserveFault(task);
                cancellationToken.ThrowIfCancellationRequested();
            }
            if (task.IsCompleted) return await task.ConfigureAwait(false);

            var cancelled = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
            using (cancellationToken.Register(() => cancelled.TrySetResult(true)))
            {
                var completed = await Task.WhenAny(task, cancelled.Task).ConfigureAwait(false);
                if (completed != task || cancellationToken.IsCancellationRequested)
                {
                    ObserveFault(task);
                    cancellationToken.ThrowIfCancellationRequested();
                }
            }
            return await task.ConfigureAwait(false);
        }

        public static async Task WithCancellation(Task task, CancellationToken cancellationToken)
        {
            if (task == null) throw new ArgumentNullException(nameof(task));
            if (!cancellationToken.CanBeCanceled)
            {
                await task.ConfigureAwait(false);
                return;
            }
            if (cancellationToken.IsCancellationRequested)
            {
                ObserveFault(task);
                cancellationToken.ThrowIfCancellationRequested();
            }
            if (task.IsCompleted)
            {
                await task.ConfigureAwait(false);
                return;
            }

            var cancelled = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
            using (cancellationToken.Register(() => cancelled.TrySetResult(true)))
            {
                var completed = await Task.WhenAny(task, cancelled.Task).ConfigureAwait(false);
                if (completed != task || cancellationToken.IsCancellationRequested)
                {
                    ObserveFault(task);
                    cancellationToken.ThrowIfCancellationRequested();
                }
            }
            await task.ConfigureAwait(false);
        }

        private static void ObserveFault(Task task)
        {
            _ = task.ContinueWith(
                t => { _ = t.Exception; },
                CancellationToken.None,
                TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously,
                TaskScheduler.Default);
        }
    }
}
