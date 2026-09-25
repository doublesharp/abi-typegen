/* Exercise the JSON-RPC parser compiled into the generated C bridge. */
#include "Token.cobol.c"

static int expect_raw(const char *response, size_t response_len, int accepted, const char *label) {
    uint8_t bytes[8] = {0};
    uint32_t length = 99;
    char error[512] = {0};
    int status = atg_cobol_json_result_hex(
        response, response_len, bytes, sizeof(bytes), &length, error, sizeof(error));
    if (accepted) {
        if (status != 0 || length != 2 || bytes[0] != 0x12 || bytes[1] != 0x34) {
            fprintf(stderr, "%s: valid result rejected: %.*s\n", label, (int)sizeof(error), error);
            return 1;
        }
    } else if (status == 0 || length != 0) {
        fprintf(stderr, "%s: invalid result accepted\n", label);
        return 1;
    }
    return 0;
}

static int expect_result(const char *response, int accepted, const char *label) {
    return expect_raw(response, strlen(response), accepted, label);
}

int main(void) {
    int failed = 0;
    failed += expect_result("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x1234\"}", 1, "valid");
    failed += expect_result("{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32000,\"data\":{\"result\":\"0x1234\"}}}", 0, "nested error result");
    failed += expect_result("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x1234\"", 0, "truncated JSON");
    failed += expect_result("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x1234\"}false", 0, "trailing JSON");
    failed += expect_result("{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":\"0x1234\"}", 0, "wrong id");
    failed += expect_result("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x12gh\"}", 0, "nonhex result");
    static const char embedded_nul[] =
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x1234\"}\0{}";
    failed += expect_raw(embedded_nul, sizeof(embedded_nul) - 1, 0, "embedded NUL");
    if (!failed) puts("COBOL bridge JSON parser passed");
    return failed ? 1 : 0;
}
