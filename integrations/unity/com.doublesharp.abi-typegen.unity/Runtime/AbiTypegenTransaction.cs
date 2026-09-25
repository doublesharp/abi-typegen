using System;
using System.Numerics;
using Nethereum.Hex.HexTypes;
using Nethereum.RPC.Eth.DTOs;

namespace AbiTypegen.Unity
{
    /// <summary>Builds exact-integer transaction inputs around calldata emitted by generated bindings.</summary>
    public static class AbiTypegenTransaction
    {
        private static readonly BigInteger Uint256ExclusiveMax = BigInteger.One << 256;

        public static TransactionInput Create(
            string contractAddress,
            string calldata,
            string from,
            BigInteger? gasLimit = null,
            BigInteger? value = null,
            BigInteger? gasPrice = null)
        {
            RequireHex(contractAddress, nameof(contractAddress), 40);
            RequireHex(calldata, nameof(calldata));
            RequireHex(from, nameof(from), 40);
            RequireUint256(gasLimit, nameof(gasLimit));
            RequireUint256(value, nameof(value));
            RequireUint256(gasPrice, nameof(gasPrice));

            var input = new TransactionInput
            {
                To = contractAddress,
                From = from,
                Data = calldata
            };
            if (gasLimit.HasValue) input.Gas = new HexBigInteger(gasLimit.Value);
            if (value.HasValue) input.Value = new HexBigInteger(value.Value);
            if (gasPrice.HasValue) input.GasPrice = new HexBigInteger(gasPrice.Value);
            return input;
        }

        private static void RequireHex(string value, string parameter, int? exactDigits = null)
        {
            if (string.IsNullOrEmpty(value) || !value.StartsWith("0x", StringComparison.OrdinalIgnoreCase) ||
                (value.Length - 2) % 2 != 0 ||
                (exactDigits.HasValue && value.Length - 2 != exactDigits.Value))
                throw new ArgumentException("Expected even-length 0x-prefixed hexadecimal data.", parameter);

            for (var index = 2; index < value.Length; index++)
            {
                var digit = value[index];
                if (!((digit >= '0' && digit <= '9') || (digit >= 'a' && digit <= 'f') ||
                      (digit >= 'A' && digit <= 'F')))
                    throw new ArgumentException("Expected hexadecimal data.", parameter);
            }
        }

        private static void RequireUint256(BigInteger? value, string parameter)
        {
            if (value.HasValue && (value.Value.Sign < 0 || value.Value >= Uint256ExclusiveMax))
                throw new ArgumentOutOfRangeException(parameter);
        }
    }
}
