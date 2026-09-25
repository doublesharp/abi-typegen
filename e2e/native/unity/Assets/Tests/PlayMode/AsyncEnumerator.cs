using System;
using System.Collections;
using System.Threading.Tasks;
using UnityEngine;

namespace AbiTypegen.Unity.E2E.Tests
{
    internal static class AsyncEnumerator
    {
        public static IEnumerator Run(Task task) => Run(task, 60f);

        public static IEnumerator Run(Task task, float timeoutSeconds)
        {
            if (task == null) throw new ArgumentNullException(nameof(task));
            if (float.IsNaN(timeoutSeconds) || timeoutSeconds < 0f)
                throw new ArgumentOutOfRangeException(nameof(timeoutSeconds));

            var started = Time.realtimeSinceStartup;
            while (!task.IsCompleted)
            {
                if (Time.realtimeSinceStartup - started >= timeoutSeconds)
                    throw new TimeoutException($"Unity async test did not complete within {timeoutSeconds} seconds.");
                yield return null;
            }
            if (task.IsCanceled) throw new OperationCanceledException();
            if (task.IsFaulted) throw task.Exception.InnerException ?? task.Exception;
        }
    }
}
