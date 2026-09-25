import com.example.contracts.NativeCases
import com.example.contracts.TupleCases
import com.example.contracts.Vault
import com.example.contracts.EdgeCases
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import org.web3j.abi.FunctionEncoder
import org.web3j.abi.datatypes.DynamicBytes
import org.web3j.abi.datatypes.Function
import org.web3j.abi.datatypes.generated.Bytes32
import org.web3j.abi.datatypes.generated.Uint256
import org.web3j.abi.datatypes.Address
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.protocol.core.DefaultBlockParameterName
import org.web3j.protocol.core.methods.response.Log
import kotlin.test.assertFails
import kotlin.test.assertFailsWith

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

    @Test
    fun typedWrapperEncodesAndDecodesOffline() {
        val owner = "0x0000000000000000000000000000000000000001"
        val call = NativeCases.encodeBalanceOf(owner)
        assertEquals(NativeCases.BALANCE_OF_SELECTOR, call.substring(0, 10))
        assertEquals(BigInteger.TEN, NativeCases.decodeBalanceOfResult("0x" + org.web3j.abi.TypeEncoder.encode(Uint256(BigInteger.TEN))))
        val transfer = NativeCases.encodeTransfer(owner, BigInteger.TEN)
        assertEquals(NativeCases.TRANSFER_SELECTOR, transfer.substring(0, 10))
        assertFails { NativeCases.decodeBalanceOfResult("0x") }
        assertFailsWith<UnsupportedOperationException> {
            NativeCases.encodeStore(NativeCases.Data(DynamicBytes(byteArrayOf(1))), List(40) { BigInteger.ONE })
        }

        val recipient = "0x0000000000000000000000000000000000000002"
        assertEquals(Vault.DEPOSIT_0_SELECTOR, Vault.encodeDeposit0(BigInteger.TEN, recipient).substring(0, 10))
        assertEquals(Vault.DEPOSIT_1_SELECTOR, Vault.encodeDeposit1(BigInteger.TEN).substring(0, 10))
        assertEquals(BigInteger.TEN, Vault.TransactionOptions(BigInteger.ONE, BigInteger.TEN, BigInteger.TEN).value)
    }

    @Test
    fun typedWrapperDecodesEventsAndErrors() {
        val from = "0x0000000000000000000000000000000000000001"
        val to = "0x0000000000000000000000000000000000000002"
        val log = Log().apply {
            topics = listOf(
                NativeCases.TRANSFER_EVENT_TOPIC,
                "0x" + org.web3j.abi.TypeEncoder.encode(Address(from)),
                "0x" + org.web3j.abi.TypeEncoder.encode(Address(to)),
            )
            data = "0x" + org.web3j.abi.TypeEncoder.encode(Uint256(BigInteger.TEN))
        }
        assertEquals(NativeCases.TransferEvent(from, to, BigInteger.TEN), NativeCases.decodeTransferEvent(log))
        val filter = NativeCases.filterTransferEvent(from, DefaultBlockParameterName.EARLIEST, DefaultBlockParameterName.LATEST, from, null)
        assertEquals(3, filter.topics.size)
        assertEquals(NativeCases.TRANSFER_EVENT_TOPIC, filter.topics[0].value)

        val encodedError = Vault.INSUFFICIENT_SHARES_ERROR_SELECTOR +
            org.web3j.abi.TypeEncoder.encode(Uint256(BigInteger.TEN)) +
            org.web3j.abi.TypeEncoder.encode(Uint256(BigInteger.ONE))
        assertEquals(BigInteger.TEN, Vault.decodeInsufficientSharesError(encodedError)?.available)
        val dynamicErrorArgs = FunctionEncoder.encodeConstructor(
            listOf(Uint256(BigInteger.ONE), DynamicBytes(byteArrayOf(1, 2, 3))),
        )
        val dynamicError = NativeCases.FAILED_ERROR_SELECTOR + dynamicErrorArgs.removePrefix("0x")
        assertEquals(BigInteger.ONE, NativeCases.decodeFailedError(dynamicError)?.index)
    }

    @Test
    fun typedWrapperDecodesNestedArraysAndTuples() {
        val row = DynamicArray(Uint256::class.java, listOf(Uint256(BigInteger.ONE), Uint256(BigInteger.TEN)))
        val matrix = DynamicArray(DynamicArray::class.java, listOf(row))
        val matrixOutput = "0x" + org.web3j.abi.TypeEncoder.encode(Uint256(32)) + org.web3j.abi.TypeEncoder.encode(matrix)
        assertEquals(listOf(listOf(BigInteger.ONE, BigInteger.TEN)), EdgeCases.decodeNestedArrayResult(matrixOutput))

        val amounts = NativeCases.Amounts(listOf(BigInteger.ONE, BigInteger.TEN))
        val tupleOutput = "0x" + org.web3j.abi.TypeEncoder.encode(Uint256(32)) + org.web3j.abi.TypeEncoder.encode(amounts)
        assertEquals(amounts, NativeCases.decodeGetAmountsResult(tupleOutput))
    }

    @Test
    fun deeplyNestedBatchTupleDecodesWithoutLosingArrayValues() {
        val original = batch(3000)
        val output = "0x" + org.web3j.abi.TypeEncoder.encode(Uint256(32)) + org.web3j.abi.TypeEncoder.encode(original)
        val decoded = NativeCases.decodeBatchValue(output)
        assertEquals(original, decoded)
    }

    @Test
    fun indexedDynamicEventExposesHashes() {
        val hash = "0x" + "ab".repeat(32)
        val log = Log().apply {
            topics = listOf(NativeCases.INDEXED_REFERENCES_EVENT_TOPIC, hash, hash, hash)
            data = "0x"
        }
        val decoded = NativeCases.decodeIndexedReferencesEvent(log)
        assertEquals(hash, decoded?.label)
        assertEquals(hash, decoded?.payload)
        assertEquals(hash, decoded?.values)
    }
}
