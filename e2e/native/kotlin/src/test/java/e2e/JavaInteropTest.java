package e2e;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.example.contracts.NativeCases;
import com.example.contracts.Vault;
import java.math.BigInteger;
import org.junit.jupiter.api.Test;

/** Generated Kotlin types are usable from Java without mangled names. */
class JavaInteropTest {
    @Test
    void gettersAndConstantsAreCallable() {
        Vault.Position position =
                new Vault.Position(BigInteger.ONE, BigInteger.TWO, "0x0000000000000000000000000000000000000003");
        assertEquals(BigInteger.TWO, position.getDepositedAt());
        assertEquals(position, new Vault.Position(BigInteger.ONE, BigInteger.TWO, position.getToken()));
        assertEquals("0xa9059cbb", NativeCases.TRANSFER_SELECTOR);
        assertEquals('[', NativeCases.JSON.charAt(0));
        NativeCases.RecordParams params = new NativeCases.RecordParams("a", "b", BigInteger.ZERO);
        assertEquals("b", params.getAddress2());
    }
}
