using System.Numerics;
using Contracts;
using Nethereum.Util;
using Nethereum.Web3;

static class NamesTest
{
    public static void Run(Web3 web3)
    {
        var binding = new NamesBinding(web3, "0x0000000000000000000000000000000000000001");
        var args = new NamesSendParams {
            Gas2 = 1, Gas22 = 2, GasPrice2 = 3, MaxFeePerGas2 = 4,
            MaxPriorityFeePerGas2 = 5, Nonce2 = 6, AmountToSend2 = 7,
            FromAddress2 = 8, ToString2 = 9, NamesSendParams2 = 10,
            Gas = 999
        };
        var signature = "send(" + string.Join(",", Enumerable.Repeat("uint256", 10)) + ")";
        var expected = Selector(signature) + string.Concat(Enumerable.Range(1, 10).Select(Word));
        Check(binding.EncodeSend(args) == expected, "ABI arguments hid inherited transaction properties");
        Check(binding.DecodeSendResult("0x" + Word(42)).NamesSendResult2 == 42, "output member shared its class name");

        Check(NamesBinding.EncodeDeployment(web3, "0x6000", new NamesConstructorParams { NamesConstructorParams2 = 7 }) == "0x6000" + Word(7), "constructor member shared its class name");
        Check(binding.EncodeConstructor2(new NamesConstructor2Params()) == Selector("Constructor()"), "constructor function DTO redeclared constructor arguments");
        Check(binding.EncodeEcho(new NamesEchoParams {
            Binding = new NamesBinding2 { Ok = true },
            Metadata = new NamesAbiMetadata2 { Ok = false },
            Constructor = new NamesConstructorParams2 { Ok = true },
            Params = new NamesEchoParams2 { Ok = false }
        }) == Selector("echo((bool),(bool),(bool),(bool))") + Word(1) + Word(0) + Word(1) + Word(0), "tuple classes collided with generated helper types");
        Check(binding.DecodeEchoResult("0x" + Word(1)).Value.Ok, "tuple result type collided with its DTO");
        Check(binding.EncodeDollar(new NamesDollarParams { Arg0 = true, Arg1 = false, Arg02 = true, Arg3 = false }) == Selector("dollar(bool,bool,bool,bool)") + Word(1) + Word(0) + Word(1) + Word(0), "normalized positional names did not stay distinct");
        Check(new NamesChangedEvent { NamesChangedEvent2 = 11 }.NamesChangedEvent2 == 11, "event member shared its class name");
        Check(binding.DecodeDeniedError(Selector("Denied(uint256)") + Word(12)).NamesDeniedError2 == 12, "error member shared its class name");

        var sdkNames = new FunctionBinding(web3, "0x0000000000000000000000000000000000000001");
        Check(sdkNames.EncodeEcho(new FunctionEchoParams { Message = new FunctionMessage2 { Ok = true }, Output = new FunctionOutput2 { Ok = false } }) == Selector("echo((bool),(bool))") + Word(1) + Word(0), "tuple types shadowed SDK base classes or attributes");
    }

    private static string Selector(string signature) => "0x" + Sha3Keccack.Current.CalculateHash(signature)[..8];
    private static string Word(int value) => new BigInteger(value).ToString("x").PadLeft(64, '0');
    private static void Check(bool condition, string message)
    {
        if (!condition) throw new Exception(message);
    }
}
