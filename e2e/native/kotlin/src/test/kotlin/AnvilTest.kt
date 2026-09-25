import com.example.contracts.Token
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import org.web3j.crypto.Credentials
import org.web3j.protocol.Web3j
import org.web3j.protocol.core.DefaultBlockParameterName
import org.web3j.protocol.core.methods.response.TransactionReceipt
import org.web3j.protocol.http.HttpService
import org.web3j.tx.RawTransactionManager

class AnvilTest {
    @Test
    fun signedTransactionsReadBackAndDecodeEvents() {
        val url = System.getenv("ATG_RPC_URL") ?: return
        val contract = requireNotNull(System.getenv("ATG_TOKEN_ADDRESS"))
        val credentials = Credentials.create(requireNotNull(System.getenv("ATG_PRIVATE_KEY")))
        val chainId = requireNotNull(System.getenv("ATG_CHAIN_ID")).toLong()
        val web3j = Web3j.build(HttpService(url))
        try {
            val manager = RawTransactionManager(web3j, credentials, chainId)
            val client = Token.Client(contract, web3j, manager)
            val gasPrice = web3j.ethGasPrice().send().gasPrice
            val options = Token.TransactionOptions(gasPrice, BigInteger.valueOf(300_000))
            val owner = credentials.address
            val amount = BigInteger.valueOf(41)
            val mint = Token.sendMint(client, owner, amount, options)
            val mintReceipt = receipt(web3j, requireNotNull(mint.transactionHash))
            assertEquals(BigInteger.ONE, mintReceipt.status?.removePrefix("0x")?.toBigInteger(16))
            assertEquals(amount, Token.callBalanceOf(client, owner))
            val transfer = mintReceipt.logs.firstNotNullOfOrNull(Token::decodeTransferEvent)
            assertNotNull(transfer)
            assertEquals(amount, transfer.amount)
            val filtered = Token.filterTransferEvent(contract, DefaultBlockParameterName.EARLIEST, DefaultBlockParameterName.LATEST, null, owner)
            assertTrue(web3j.ethGetLogs(filtered).send().logs.isNotEmpty())

            val approve = Token.sendApprove(client, owner, BigInteger.TEN, options)
            val approveReceipt = receipt(web3j, requireNotNull(approve.transactionHash))
            assertEquals(BigInteger.TEN, Token.callAllowance(client, owner, owner))
            val approval = approveReceipt.logs.firstNotNullOfOrNull(Token::decodeApprovalEvent)
            assertNotNull(approval)
            assertEquals(BigInteger.TEN, approval.amount)
        } finally {
            web3j.shutdown()
        }
    }

    private fun receipt(web3j: Web3j, hash: String): TransactionReceipt {
        repeat(30) {
            val response = web3j.ethGetTransactionReceipt(hash).send()
            if (response.transactionReceipt.isPresent) return response.transactionReceipt.get()
            Thread.sleep(100)
        }
        error("transaction receipt not available: $hash")
    }
}
