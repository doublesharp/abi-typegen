import com.example.contracts.Overloads
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertEquals
import org.web3j.abi.datatypes.DynamicBytes
import org.web3j.abi.datatypes.generated.Bytes32

class OverloadsTest {
    @Test
    fun aliasesEncodeOriginalSelectors() {
        val address = "0x0000000000000000000000000000000000000001"
        val node = Bytes32(ByteArray(32) { 2 })
        val cases = listOf(
            Triple(Overloads.SAFE_TRANSFER_FROM_0_SIGNATURE, "safeTransferFrom(address,address,uint256)", Overloads.encodeSafeTransferFrom0(address, address, BigInteger.TEN) to "0x42842e0e"),
            Triple(Overloads.SAFE_TRANSFER_FROM_1_SIGNATURE, "safeTransferFrom(address,address,uint256,bytes)", Overloads.encodeSafeTransferFrom1(address, address, BigInteger.TEN, DynamicBytes(byteArrayOf(1))) to "0xb88d4fde"),
            Triple(Overloads.ATTESTATIONS_0_SIGNATURE, "attestations(bytes32,bytes32)", Overloads.encodeAttestations0(node, node) to "0x92abd17b"),
            Triple(Overloads.ATTESTATIONS_1_SIGNATURE, "attestations(bytes32)", Overloads.encodeAttestations1(node) to "0x940992a3"),
            Triple(Overloads.SET_ADDR_0_SIGNATURE, "setAddr(bytes32,uint256,bytes)", Overloads.encodeSetAddr0(node, BigInteger.valueOf(60), DynamicBytes(byteArrayOf(2))) to "0x8b95dd71"),
            Triple(Overloads.SET_ADDR_1_SIGNATURE, "setAddr(bytes32,address)", Overloads.encodeSetAddr1(node, address) to "0xd5fa2b00"),
        )
        for ((signature, expectedSignature, encoded) in cases) {
            assertEquals(expectedSignature, signature)
            assertEquals(encoded.second, encoded.first.take(10))
        }
    }
}
