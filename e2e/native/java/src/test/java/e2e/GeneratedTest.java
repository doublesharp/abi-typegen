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
    @Test void integerExtremesAndNamedResultsRemainExact() {
        var unsigned = EdgeCases.decodeLargeIntsResult("0x"
            + "0".repeat(48) + "f".repeat(16)
            + "0".repeat(32) + "f".repeat(32)
            + "f".repeat(64));
        assertEquals(BigInteger.ONE.shiftLeft(64).subtract(BigInteger.ONE), unsigned.a());
        assertEquals(BigInteger.ONE.shiftLeft(128).subtract(BigInteger.ONE), unsigned.b());
        assertEquals(BigInteger.ONE.shiftLeft(256).subtract(BigInteger.ONE), unsigned.c());
        var signed = EdgeCases.decodeSignedIntsResult("0x"
            + "f".repeat(62) + "80"
            + "f".repeat(52) + "800000000000"
            + "8" + "0".repeat(63));
        assertEquals(BigInteger.valueOf(-128), signed.a());
        assertEquals(BigInteger.ONE.shiftLeft(47).negate(), signed.b());
        assertEquals(BigInteger.ONE.shiftLeft(255).negate(), signed.c());
        var multiple = EdgeCases.decodeMultiReturnResult("0x"
            + "0".repeat(63) + "7" + "f".repeat(64) + "0".repeat(63) + "1");
        assertEquals(BigInteger.valueOf(7), multiple.count());
        assertEquals(BigInteger.ONE.shiftLeft(256).subtract(BigInteger.ONE), multiple.total());
        assertTrue(multiple.flag());
    }

    @Test void dynamicBytesAndFixedByteOutputsDecodeTheirDeclaredLengths() {
        var dynamic = EdgeCases.decodeDynamicBytesResult("0x"
            + "0".repeat(62) + "20" + "0".repeat(63) + "3"
            + "aabbcc" + "0".repeat(58));
        assertArrayEquals(new byte[]{(byte)0xaa, (byte)0xbb, (byte)0xcc}, dynamic.getValue());
        var fixed = EdgeCases.decodeFixedBytesResult("0x"
            + "aa" + "0".repeat(62) + "bb".repeat(16) + "0".repeat(32)
            + "cc".repeat(32));
        assertArrayEquals(new byte[]{(byte)0xaa}, fixed.a().getValue());
        assertArrayEquals(java.util.HexFormat.of().parseHex("bb".repeat(16)), fixed.b().getValue());
        assertArrayEquals(java.util.HexFormat.of().parseHex("cc".repeat(32)), fixed.c().getValue());
    }

    @Test void voidResultsRejectUnexpectedReturnBytes() {
        assertDoesNotThrow(() -> EdgeCases.decodeResetResult("0x"));
        assertDoesNotThrow(() -> EdgeCases.decodeResetResult(""));
        assertThrows(IllegalArgumentException.class, () -> EdgeCases.decodeResetResult("0x" + "0".repeat(64)));
    }

    @Test void malformedLayoutsFailInsteadOfReturningPartialValues() {
        assertThrows(IllegalArgumentException.class, () -> Token.decodeBalanceOfResult("00"));
        assertThrows(IllegalArgumentException.class, () -> Token.decodeBalanceOfResult("0x0"));
        // web3j reports truncated scalar words as bounds errors.
        assertThrows(IndexOutOfBoundsException.class, () -> Token.decodeBalanceOfResult("0x"));
        assertThrows(ArithmeticException.class, () -> EdgeCases.decodeNestedArrayResult("0x" + "f".repeat(64)));
        assertThrows(IndexOutOfBoundsException.class, () -> EdgeCases.decodeDynamicBytesResult("0x" + "0".repeat(62) + "20"));
    }

    @Test void eventTopicGatingAndAnonymousLogsUseCorrectPositions() {
        Log transfer = new Log();
        transfer.setData("0x");
        transfer.setTopics(List.of());
        assertNull(Token.decodeTransferEvent(transfer));
        transfer.setTopics(List.of("0x" + "f".repeat(64)));
        assertNull(Token.decodeTransferEvent(transfer));
        transfer.setData("0x" + "0".repeat(64));
        transfer.setTopics(List.of("0x" + "f".repeat(64), "0x" + "0".repeat(64), "0x" + "0".repeat(64)));
        assertNull(Token.decodeTransferEvent(transfer));
        transfer.setTopics(List.of(Token.TRANSFER_EVENT_TOPIC));
        assertNull(Token.decodeTransferEvent(transfer));
        transfer.setTopics(List.of(Token.TRANSFER_EVENT_TOPIC, "0x" + "0".repeat(64), "0x" + "0".repeat(64), "0x" + "0".repeat(64)));
        assertNull(Token.decodeTransferEvent(transfer));
        Log anonymous = new Log();
        anonymous.setTopics(List.of());
        anonymous.setData("0x" + "0".repeat(62) + "20" + "0".repeat(63) + "2" + "6869" + "0".repeat(60));
        assertEquals("hi", EdgeCases.decodeDebugLogEvent(anonymous).message());
        anonymous.setTopics(List.of("0x" + "0".repeat(64)));
        assertNull(EdgeCases.decodeDebugLogEvent(anonymous));
    }

    @Test void errorDecodersMatchSelectorsAndSupportZeroArguments() {
        assertNull(Token.decodeInvalidRecipientError("0x00000000"));
        assertNotNull(Token.decodeInvalidRecipientError(Token.INVALID_RECIPIENT_ERROR_SELECTOR));
        assertNotNull(Token.decodeInvalidRecipientError(Token.INVALID_RECIPIENT_ERROR_SELECTOR.toUpperCase(java.util.Locale.ROOT)));
        assertNull(Vault.decodeInsufficientSharesError("0x00000000"));
        assertThrows(IndexOutOfBoundsException.class, () -> Vault.decodeInsufficientSharesError(Vault.INSUFFICIENT_SHARES_ERROR_SELECTOR));
    }

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
