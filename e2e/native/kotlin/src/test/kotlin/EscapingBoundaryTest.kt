import com.example.contracts.CommentCases
import com.example.contracts.Contract
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EscapingBoundaryTest {
    @Test
    fun commentTextDoesNotHideTheGeneratedNamespace() {
        assertEquals("read(bool,uint256,bool)", CommentCases.READ_SIGNATURE)
        val params = CommentCases.ReadParams(true, BigInteger.TEN, false)
        assertEquals(true, params.`_`)
        assertEquals(BigInteger.TEN, params.`___`)
        assertEquals(false, params.`when`)
        assertTrue(CommentCases.JSON.contains("price ${'$'}5\u0085\u009f😀"))
    }

    @Test
    fun underscoreContractRetainsItsAbi() {
        assertEquals("[]", Contract.JSON)
    }
}
