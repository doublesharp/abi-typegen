package e2e;

import com.example.contracts.NativeCases;
import com.example.contracts.EdgeCases;
import com.example.contracts.Token;
import com.example.contracts.Vault;
import java.math.BigInteger;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.web3j.abi.TypeEncoder;
import org.web3j.abi.datatypes.DynamicBytes;
import org.web3j.abi.datatypes.DynamicArray;
import org.web3j.abi.datatypes.generated.Bytes32;
import org.web3j.abi.datatypes.generated.Uint256;
import org.web3j.protocol.core.DefaultBlockParameterName;
import org.web3j.protocol.core.methods.response.Log;
import static org.junit.jupiter.api.Assertions.*;

class GeneratedTest {
    @Test void offlineCallsResultsErrorsAndEvents() {
        String owner = "0x0000000000000000000000000000000000000001";
        assertEquals(Token.BALANCE_OF_SELECTOR, Token.encodeBalanceOf(owner).substring(0, 10));
        assertEquals(BigInteger.TEN, Token.decodeBalanceOfResult("0x" + TypeEncoder.encode(new Uint256(BigInteger.TEN))));
        assertEquals(Token.TRANSFER_SELECTOR, Token.encodeTransfer(owner, BigInteger.TEN).substring(0, 10));
        assertThrows(UnsupportedOperationException.class, () -> NativeCases.encodeStore(
            new NativeCases.Data(new DynamicBytes(new byte[]{1})), java.util.Collections.nCopies(40, BigInteger.ONE)));
        String error = Vault.INSUFFICIENT_SHARES_ERROR_SELECTOR + TypeEncoder.encode(new Uint256(BigInteger.TEN)) + TypeEncoder.encode(new Uint256(BigInteger.ONE));
        assertEquals(BigInteger.TEN, Vault.decodeInsufficientSharesError(error).available());
        Log log = new Log();
        log.setTopics(List.of(Token.TRANSFER_EVENT_TOPIC, "0x" + TypeEncoder.encode(new org.web3j.abi.datatypes.Address(owner)), "0x" + TypeEncoder.encode(new org.web3j.abi.datatypes.Address(owner))));
        log.setData("0x" + TypeEncoder.encode(new Uint256(BigInteger.TEN)));
        assertEquals(BigInteger.TEN, Token.decodeTransferEvent(log).amount());
        assertEquals(3, Token.filterTransferEvent(owner, DefaultBlockParameterName.EARLIEST, DefaultBlockParameterName.LATEST, owner, null).getTopics().size());
    }

    @Test void nestedTupleAndArrayDecode() {
        String target = "0x0000000000000000000000000000000000000001";
        var call = new NativeCases.Call3Value(target, true, BigInteger.TEN, new DynamicBytes(new byte[]{1, 2, 3}));
        var grid = new NativeCases.Grid(List.of(List.of(BigInteger.ONE, BigInteger.TWO), List.of()), List.of(true, false), List.of(new Bytes32(new byte[32])));
        var batch = new NativeCases.Batch(List.of(call), grid, BigInteger.valueOf(3000), BigInteger.valueOf(-5));
        String output = "0x" + TypeEncoder.encode(new Uint256(32)) + TypeEncoder.encode(batch);
        var decoded = NativeCases.decodeBatchValue(output);
        assertEquals(BigInteger.TEN, decoded.calls.get(0).value_);
        assertEquals(List.of(BigInteger.ONE, BigInteger.TWO), decoded.grid.rows.get(0));
        assertEquals(BigInteger.valueOf(3000), decoded.fee);

        var row = new DynamicArray<>(Uint256.class, List.of(new Uint256(BigInteger.ONE), new Uint256(BigInteger.TEN)));
        var matrix = new DynamicArray<>(DynamicArray.class, List.of(row));
        String matrixOutput = "0x" + TypeEncoder.encode(new Uint256(32)) + TypeEncoder.encode(matrix);
        assertEquals(List.of(List.of(BigInteger.ONE, BigInteger.TEN)), EdgeCases.decodeNestedArrayResult(matrixOutput));

        String hash = "0x" + "ab".repeat(32);
        Log log = new Log();
        log.setTopics(List.of(NativeCases.INDEXED_REFERENCES_EVENT_TOPIC, hash, hash, hash));
        log.setData("0x");
        var event = NativeCases.decodeIndexedReferencesEvent(log);
        assertEquals(hash, event.label());
        assertEquals(hash, event.payload());
        assertEquals(hash, event.values());
    }
}
