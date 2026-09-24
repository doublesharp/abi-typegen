import com.example.contracts.NativeCases
import com.example.contracts.TupleCases
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import org.web3j.abi.FunctionEncoder
import org.web3j.abi.datatypes.DynamicBytes
import org.web3j.abi.datatypes.Function
import org.web3j.abi.datatypes.generated.Bytes32

class GeneratedTest {
    @Test
    fun packedMarketDecodesFromFunctionOutput() {
        val market = NativeCases.PackedMarket(Bytes32(ByteArray(32) { 1 }), Bytes32(ByteArray(32) { 2 }), BigInteger.TEN)
        val encoded = org.web3j.abi.TypeEncoder.encode(market)
        val decoded = org.web3j.abi.FunctionReturnDecoder.decode(
            encoded,
            Function("getMarket", emptyList(), listOf(org.web3j.abi.TypeReference.create(NativeCases.PackedMarket::class.java))).outputParameters,
        ).single() as NativeCases.PackedMarket
        assertEquals(market, decoded)
    }

    @Test
    fun arrayTupleDecodesFromFunctionOutput() {
        val amounts = NativeCases.Amounts(listOf(BigInteger.ONE, BigInteger.TEN))
        val encoded = org.web3j.abi.TypeEncoder.encode(amounts)
        // A dynamic return value starts with an offset to its tuple payload.
        val output = org.web3j.abi.TypeEncoder.encode(org.web3j.abi.datatypes.generated.Uint256(32)) + encoded
        val decoded = org.web3j.abi.FunctionReturnDecoder.decode(
            output,
            Function("getAmounts", emptyList(), listOf(org.web3j.abi.TypeReference.create(NativeCases.Amounts::class.java))).outputParameters,
        ).single() as NativeCases.Amounts
        assertEquals(amounts, decoded)
    }

    private fun batch(fee: Long) = NativeCases.Batch(
        calls = listOf(
            NativeCases.Call3Value(
                target = "0x0000000000000000000000000000000000000001",
                allowFailure = true,
                value_ = BigInteger.TEN,
                callData = DynamicBytes(byteArrayOf(1, 2, 3)),
            ),
        ),
        grid = NativeCases.Grid(
            rows = listOf(listOf(BigInteger.ONE, BigInteger.TWO), listOf()),
            flags = listOf(true, false),
            tags = listOf(Bytes32(ByteArray(32) { 7 })),
        ),
        fee = BigInteger.valueOf(fee),
        delta = BigInteger.valueOf(-5),
    )

    @Test
    fun structsEncodeWithTheGeneratedSelector() {
        val encoded = FunctionEncoder.encode(Function("aggregate", listOf(batch(3000)), emptyList()))
        assertEquals(NativeCases.AGGREGATE_SELECTOR, encoded.substring(0, 10))

        val deposit = FunctionEncoder.encode(
            Function(
                "deposit",
                listOf(TupleCases.TupleAccountPosition("0x0000000000000000000000000000000000000002")),
                emptyList(),
            ),
        )
        assertEquals(TupleCases.DEPOSIT_0_SELECTOR, deposit.substring(0, 10))
    }

    @Test
    fun valuesCompareByContent() {
        assertEquals(batch(3000), batch(3000))
        assertEquals(batch(3000).hashCode(), batch(3000).hashCode())
        assertNotEquals(batch(3000), batch(500))
    }

    @Test
    fun abiIsEmbedded() {
        assert(NativeCases.JSON.startsWith("[{"))
    }
}
