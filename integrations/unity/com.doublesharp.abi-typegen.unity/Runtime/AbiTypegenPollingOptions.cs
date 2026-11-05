using System;
using System.Numerics;
using System.Threading;

namespace AbiTypegen.Unity
{
    /// <summary>Bounds HTTP polling and defines inclusion depth (one confirmation is the inclusion block).</summary>
    public sealed class AbiTypegenPollingOptions
    {
        /// <summary>Required canonical block depth.</summary>
        public int Confirmations { get; set; } = 1;
        /// <summary>Delay between polls.</summary>
        public TimeSpan PollInterval { get; set; } = TimeSpan.FromSeconds(1);
        /// <summary>Maximum duration of a receipt wait, a single event poll, or an entire event watch.</summary>
        public TimeSpan Timeout { get; set; } = TimeSpan.FromMinutes(2);
        /// <summary>Maximum contiguous block range requested by each event poll.</summary>
        public int MaxBlocksPerPoll { get; set; } = 1000;

        internal AbiTypegenPollingOptions Snapshot()
        {
            if (Confirmations < 1) throw new ArgumentOutOfRangeException(nameof(Confirmations));
            if (PollInterval.TotalMilliseconds < 1 || PollInterval.TotalMilliseconds > int.MaxValue)
                throw new ArgumentOutOfRangeException(nameof(PollInterval));
            if (Timeout.TotalMilliseconds < 1 || Timeout.TotalMilliseconds > int.MaxValue)
                throw new ArgumentOutOfRangeException(nameof(Timeout));
            if (MaxBlocksPerPoll < 1) throw new ArgumentOutOfRangeException(nameof(MaxBlocksPerPoll));
            return new AbiTypegenPollingOptions { Confirmations = Confirmations, PollInterval = PollInterval,
                Timeout = Timeout, MaxBlocksPerPoll = MaxBlocksPerPoll };
        }
    }

    /// <summary>An in-memory event cursor. Reorgs behind an acknowledged range fail explicitly.</summary>
    public sealed class AbiTypegenEventCursor
    {
        /// <summary>Creates a cursor at an explicit inclusive start block.</summary>
        public AbiTypegenEventCursor(BigInteger startBlock)
        {
            AbiTypegenRpcOperations.RequireQuantity(startBlock, nameof(startBlock));
            NextBlock = startBlock;
        }
        /// <summary>First block that has not yet been consumed.</summary>
        public BigInteger NextBlock { get; internal set; }
        internal BigInteger? CheckpointBlock;
        internal string CheckpointHash;
        internal readonly SemaphoreSlim Gate = new SemaphoreSlim(1, 1);
    }
}
