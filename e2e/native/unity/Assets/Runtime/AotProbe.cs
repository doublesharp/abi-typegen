using System;
using System.Numerics;
using Contracts;
using Nethereum.ABI.FunctionEncoding;
using Nethereum.ABI.Model;
using Nethereum.Contracts;
using Nethereum.Util;
using UnityEngine;

namespace AbiTypegen.Unity.E2E
{
    /// <summary>Runs exact ABI codec assertions in a standalone Mono or IL2CPP player.</summary>
    public sealed class AotProbe : MonoBehaviour
    {
        private void Start()
        {
            try
            {
                var exact = (BigInteger.One << 200) + 7;
                if (TokenAbiMetadata.ABI.Length == 0 || CodecCasesAbiMetadata.ABI.Length == 0)
                    throw new InvalidOperationException("Generated ABI metadata is empty.");

                var selector = new Sha3Keccack().CalculateHash("echoUint(uint256)").Substring(0, 8);
                var input = new CodecCasesEchoUintParams { Value = exact };
                var calldata = new FunctionCallEncoder().EncodeRequest(input, typeof(CodecCasesEchoUintParams), selector);
                if (!calldata.StartsWith("0x" + selector, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidOperationException("AOT calldata selector mismatch.");

                var output = "0x" + calldata.Substring(10);
                var decoded = new FunctionCallDecoder().DecodeFunctionOutput(new CodecCasesEchoUintResult(), output);
                if (decoded.Value != exact)
                    throw new InvalidOperationException("AOT uint256 codec lost precision.");

                var tuple = new CodecCasesItem
                {
                    Amount = exact,
                    Owner = "0x0000000000000000000000000000000000000001"
                };
                var tupleSelector = new Sha3Keccack().CalculateHash("echoTuple((uint256,address))").Substring(0, 8);
                var tupleCall = new FunctionCallEncoder().EncodeRequest(
                    new CodecCasesEchoTupleParams { Item = tuple }, typeof(CodecCasesEchoTupleParams), tupleSelector);
                var tupleResult = new FunctionCallDecoder().DecodeFunctionOutput(
                    new CodecCasesEchoTupleResult(), "0x" + tupleCall.Substring(10));
                if (tupleResult.Item.Amount != tuple.Amount ||
                    !string.Equals(tupleResult.Item.Owner, tuple.Owner, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidOperationException("AOT named tuple codec mismatch.");

                var errorSelector = new Sha3Keccack().CalculateHash("Denied(address,uint256,uint256)").Substring(0, 8);
                var errorData = new FunctionCallEncoder().EncodeRequest(
                    errorSelector,
                    new[]
                    {
                        new Parameter("address", "owner", 1),
                        new Parameter("uint256", "available", 2),
                        new Parameter("uint256", "required", 3)
                    },
                    tuple.Owner, BigInteger.Zero, exact);
                var error = new Nethereum.Contracts.Error(typeof(CodecCasesDeniedError))
                    .DecodeExceptionEncodedData<CodecCasesDeniedError>(errorData);
                if (error.Required != exact || error.Available != BigInteger.Zero ||
                    !string.Equals(error.Owner, tuple.Owner, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidOperationException("AOT custom error codec mismatch.");

                Debug.Log("abi-typegen Unity AOT codec passed");
                Application.Quit(0);
            }
            catch (Exception error)
            {
                Debug.LogException(error);
                Application.Quit(1);
            }
        }
    }
}
