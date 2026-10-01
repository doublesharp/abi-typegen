package e2e;

import com.example.contracts.RecordNames;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.web3j.protocol.core.methods.response.Log;
import static org.junit.jupiter.api.Assertions.*;

class RecordNamesTest {
    @Test void embeddedMetadataPreservesUnicodeControlCharacters() throws Exception {
        var abi = tools.jackson.databind.json.JsonMapper.builder().build().readTree(RecordNames.ABI);
        assertEquals("delete\u007fnext\u0085line", abi.get(1).get("x-note").asString());
    }

    @Test void escapedObjectMethodNamesRemainReadableAcrossDecodedRecords() {
        String payload = "0".repeat(63) + "1";
        payload = payload.repeat(9);
        var result = RecordNames.decodeEchoResult("0x" + payload);
        assertTrue(result.hashCode_());
        assertTrue(result.hashCode_2());
        assertTrue(result.getClass_());
        assertTrue(result.clone_());
        assertTrue(result.finalize_());
        assertTrue(result.notify_());
        assertTrue(result.notifyAll_());
        assertTrue(result.toString_());
        assertTrue(result.wait_());
        var error = RecordNames.decodeDeniedError(RecordNames.DENIED_ERROR_SELECTOR + payload);
        assertTrue(error.hashCode_());
        assertTrue(error.hashCode_2());
        var log = new Log();
        log.setTopics(List.of(RecordNames.FIELDS_EVENT_TOPIC));
        log.setData("0x" + payload);
        var event = RecordNames.decodeFieldsEvent(log);
        assertTrue(event.hashCode_());
        assertTrue(event.hashCode_2());
        assertTrue(RecordNames.encodeEcho(true, true, true, true, true, true, true, true, true)
            .startsWith(RecordNames.ECHO_SELECTOR));
    }
}
