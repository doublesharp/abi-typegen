package e2e;

import com.example.contracts.Token;
import java.math.BigInteger;
import org.junit.jupiter.api.Test;
import org.web3j.crypto.Credentials;
import org.web3j.protocol.Web3j;
import org.web3j.protocol.core.DefaultBlockParameterName;
import org.web3j.protocol.core.methods.response.TransactionReceipt;
import org.web3j.protocol.http.HttpService;
import org.web3j.tx.RawTransactionManager;
import static org.junit.jupiter.api.Assertions.*;

class AnvilTest {
    @Test void signedTransactionsReadBackAndDecodeEvents() throws Exception {
        String url = System.getenv("ATG_RPC_URL");
        if (url == null) return;
        String contract = System.getenv("ATG_TOKEN_ADDRESS");
        var credentials = Credentials.create(System.getenv("ATG_PRIVATE_KEY"));
        long chainId = Long.parseLong(System.getenv("ATG_CHAIN_ID"));
        var web3j = Web3j.build(new HttpService(url));
        try {
            var client = new Token.Client(contract, web3j, new RawTransactionManager(web3j, credentials, chainId));
            var options = new Token.TransactionOptions(web3j.ethGasPrice().send().getGasPrice(), BigInteger.valueOf(300_000));
            String owner = credentials.getAddress();
            var amount = BigInteger.valueOf(41);
            var mint = Token.sendMint(client, owner, amount, options);
            var mintReceipt = receipt(web3j, mint.getTransactionHash());
            assertEquals(BigInteger.ONE, new BigInteger(mintReceipt.getStatus().substring(2), 16));
            assertEquals(amount, Token.callBalanceOf(client, owner));
            var transfer = mintReceipt.getLogs().stream().map(Token::decodeTransferEvent).filter(java.util.Objects::nonNull).findFirst().orElseThrow();
            assertEquals(amount, transfer.amount());
            var filter = Token.filterTransferEvent(contract, DefaultBlockParameterName.EARLIEST, DefaultBlockParameterName.LATEST, null, owner);
            assertFalse(web3j.ethGetLogs(filter).send().getLogs().isEmpty());

            var approve = Token.sendApprove(client, owner, BigInteger.TEN, options);
            var approveReceipt = receipt(web3j, approve.getTransactionHash());
            assertEquals(BigInteger.TEN, Token.callAllowance(client, owner, owner));
            var approval = approveReceipt.getLogs().stream().map(Token::decodeApprovalEvent).filter(java.util.Objects::nonNull).findFirst().orElseThrow();
            assertEquals(BigInteger.TEN, approval.amount());
        } finally {
            web3j.shutdown();
        }
    }
    private TransactionReceipt receipt(Web3j web3j, String hash) throws Exception {
        for (int attempt = 0; attempt < 30; attempt++) {
            var found = web3j.ethGetTransactionReceipt(hash).send().getTransactionReceipt();
            if (found.isPresent()) return found.get();
            Thread.sleep(100);
        }
        throw new IllegalStateException("transaction receipt not available: " + hash);
    }
}
