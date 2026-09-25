using System;
using System.Linq;
using System.Numerics;
using Contracts;
using Nethereum.ABI.FunctionEncoding;
using Nethereum.ABI.Model;
using Nethereum.Contracts;
using Nethereum.Util;
using NUnit.Framework;

namespace AbiTypegen.Unity.E2E.Tests
{
    public sealed class GeneratedBindingsEditModeTests
    {
        private const string Owner = "0x0000000000000000000000000000000000000001";

        [Test]
        public void MetadataAndGeneratedNamesAreDeterministic()
        {
            StringAssert.Contains("\"echoUint\"", CodecCasesAbiMetadata.ABI);
            StringAssert.Contains("\"Denied\"", CodecCasesAbiMetadata.ABI);
            Assert.That(typeof(CodecCasesOverloadedUint256Params).Name, Is.EqualTo("CodecCasesOverloadedUint256Params"));
            Assert.That(typeof(CodecCasesOverloadedAddressParams).Name, Is.EqualTo("CodecCasesOverloadedAddressParams"));
            Assert.That(typeof(CodecCasesItem).Name, Is.EqualTo("CodecCasesItem"));
        }

        [Test]
        public void MetadataOnlyFixtureHasNoCallableBinding()
        {
            var binding = typeof(CodecCasesAbiMetadata).Assembly.GetType("Contracts.CodecCasesBinding", false);
            Assert.That(binding, Is.Null, "wrappers=false must not silently emit callable wrappers");
        }

        [Test]
        public void Uint256Above128BitsRoundTripsExactlyThroughNethereumCodec()
        {
            var value = (BigInteger.One << 200) + BigInteger.Parse("123456789012345678901234567890");
            Assert.That(value, Is.GreaterThan(BigInteger.One << 128));

            var input = new CodecCasesEchoUintParams { Value = value };
            var calldata = EncodeFunction(input, "echoUint(uint256)");
            var output = AsReturnData(calldata);
            var decoded = new FunctionCallDecoder().DecodeFunctionOutput(new CodecCasesEchoUintResult(), output);

            Assert.That(decoded.Value, Is.EqualTo(value));
        }

        [Test]
        public void OverloadsUseDistinctCanonicalSelectorsAndGeneratedNames()
        {
            var uintCall = EncodeFunction(
                new CodecCasesOverloadedUint256Params { Value = BigInteger.One },
                "overloaded(uint256)");
            var addressCall = EncodeFunction(
                new CodecCasesOverloadedAddressParams { Value = Owner },
                "overloaded(address)");

            Assert.That(uintCall.Substring(0, 10), Is.Not.EqualTo(addressCall.Substring(0, 10)));
            Assert.That(typeof(CodecCasesOverloadedUint256Params), Is.Not.EqualTo(typeof(CodecCasesOverloadedAddressParams)));
        }

        [Test]
        public void NamedTupleRoundTripsThroughNethereumCodec()
        {
            var value = new CodecCasesItem
            {
                Amount = (BigInteger.One << 180) + 17,
                Owner = Owner
            };
            var calldata = EncodeFunction(new CodecCasesEchoTupleParams { Item = value }, "echoTuple((uint256,address))");
            var decoded = new FunctionCallDecoder().DecodeFunctionOutput(
                new CodecCasesEchoTupleResult(),
                AsReturnData(calldata));

            Assert.That(decoded.Item.Amount, Is.EqualTo(value.Amount));
            Assert.That(decoded.Item.Owner, Is.EqualTo(Owner).IgnoreCase);
        }

        [Test]
        public void DynamicArrayRoundTripsWithoutFloatingPointConversion()
        {
            var values = new[]
            {
                BigInteger.Zero,
                BigInteger.One,
                (BigInteger.One << 200) + 99
            }.ToList();
            var calldata = EncodeFunction(new CodecCasesEchoArrayParams { Values = values }, "echoArray(uint256[])");
            var decoded = new FunctionCallDecoder().DecodeFunctionOutput(
                new CodecCasesEchoArrayResult(),
                AsReturnData(calldata));

            CollectionAssert.AreEqual(values, decoded.Values);
        }

        [Test]
        public void OutOfRangeUint256FailsInsteadOfTruncating()
        {
            var tooLarge = BigInteger.One << 256;
            Assert.That(
                () => EncodeFunction(new CodecCasesEchoUintParams { Value = tooLarge }, "echoUint(uint256)"),
                Throws.Exception);
        }

        [Test]
        public void CustomErrorDecodesThroughNethereumErrorCodec()
        {
            var selector = Selector("Denied(address,uint256,uint256)");
            var encoder = new FunctionCallEncoder();
            var encoded = encoder.EncodeRequest(
                selector,
                new[]
                {
                    new Parameter("address", "owner", 1),
                    new Parameter("uint256", "available", 2),
                    new Parameter("uint256", "required", 3)
                },
                Owner,
                BigInteger.Parse("340282366920938463463374607431768211457"),
                BigInteger.Parse("680564733841876926926749214863536422913"));

            var decoded = new Nethereum.Contracts.Error(typeof(CodecCasesDeniedError))
                .DecodeExceptionEncodedData<CodecCasesDeniedError>(encoded);

            Assert.That(decoded.Owner, Is.EqualTo(Owner).IgnoreCase);
            Assert.That(decoded.Available, Is.EqualTo(BigInteger.Parse("340282366920938463463374607431768211457")));
            Assert.That(decoded.Required, Is.EqualTo(BigInteger.Parse("680564733841876926926749214863536422913")));
        }

        [Test]
        public void TransactionBuilderRejectsNegativeExactValues()
        {
            Assert.That(
                () => AbiTypegen.Unity.AbiTypegenTransaction.Create(
                    "0x0000000000000000000000000000000000000002",
                    "0x12345678",
                    Owner,
                    value: new BigInteger(-1)),
                Throws.TypeOf<ArgumentOutOfRangeException>());
        }

        [Test]
        public void EventDtoRetainsIndexedMetadata()
        {
            var attributes = typeof(CodecCasesChangedEvent).GetCustomAttributes(false);
            Assert.That(attributes.Any(attribute => attribute is Nethereum.ABI.FunctionEncoding.Attributes.EventAttribute), Is.True);

            var ownerProperty = typeof(CodecCasesChangedEvent).GetProperty(nameof(CodecCasesChangedEvent.Owner));
            var parameter = (Nethereum.ABI.FunctionEncoding.Attributes.ParameterAttribute)
                ownerProperty.GetCustomAttributes(typeof(Nethereum.ABI.FunctionEncoding.Attributes.ParameterAttribute), false).Single();
            Assert.That(parameter.Parameter.Indexed, Is.True);
        }

        private static string EncodeFunction<T>(T value, string canonicalSignature)
        {
            return new FunctionCallEncoder().EncodeRequest(value, typeof(T), Selector(canonicalSignature));
        }

        private static string Selector(string canonicalSignature)
        {
            return new Sha3Keccack().CalculateHash(canonicalSignature).Substring(0, 8);
        }

        private static string AsReturnData(string calldata)
        {
            if (calldata == null || calldata.Length < 10 || !calldata.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
                throw new ArgumentException("Expected selector-prefixed calldata.", nameof(calldata));
            return "0x" + calldata.Substring(10);
        }
    }
}
