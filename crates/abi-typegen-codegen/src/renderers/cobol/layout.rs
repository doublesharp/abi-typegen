//! C bridge for generated COBOL bindings.
//!
//! The bridge exposes a deliberately flat ABI: fixed COBOL buffers plus 32-bit
//! lengths and status codes. It owns every `atg_value` / `atg_result` lifecycle,
//! so COBOL never needs to understand opaque Rust pointers.

use super::PocFunction;
use abi_typegen_core::types::ContractIr;

fn c_literal(value: &str) -> String {
    serde_json::to_string(value).expect("string serializes")
}

fn c_ident(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("contract");
    }
    if out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, '_');
    }
    out
}

const SUPPORT: &str = r#"
#include "abi_typegen.h"
#include <ctype.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef ATG_COBOL_WITH_CURL
#include <curl/curl.h>
#include <json-c/json.h>
#endif

static void atg_cobol_error(char *out, uint32_t cap, const char *message) {
    if (!out || cap == 0) return;
    memset(out, ' ', cap);
    if (!message) return;
    size_t n = strlen(message);
    if (n > cap) n = cap;
    memcpy(out, message, n);
}

static uint32_t atg_cobol_trim_len(const char *text, uint32_t len) {
    if (!text) return 0;
    while (len && (text[len - 1] == ' ' || text[len - 1] == '\0' ||
                   text[len - 1] == '\r' || text[len - 1] == '\n' ||
                   text[len - 1] == '\t')) {
        --len;
    }
    return len;
}

static int atg_cobol_hex_nibble(unsigned char c) {
    if (c >= '0' && c <= '9') return (int)(c - '0');
    if (c >= 'a' && c <= 'f') return (int)(c - 'a') + 10;
    if (c >= 'A' && c <= 'F') return (int)(c - 'A') + 10;
    return -1;
}

static int atg_cobol_parse_address(
    const char *text,
    uint32_t text_len,
    uint8_t out[20],
    char *error,
    uint32_t error_cap
) {
    text_len = atg_cobol_trim_len(text, text_len);
    uint32_t at = 0;
    if (text_len >= 2 && text[0] == '0' && (text[1] == 'x' || text[1] == 'X')) at = 2;
    if (text_len - at != 40) {
        atg_cobol_error(error, error_cap, "address must contain exactly 20 bytes");
        return -1;
    }
    for (uint32_t i = 0; i < 20; ++i) {
        int hi = atg_cobol_hex_nibble((unsigned char)text[at + i * 2]);
        int lo = atg_cobol_hex_nibble((unsigned char)text[at + i * 2 + 1]);
        if (hi < 0 || lo < 0) {
            atg_cobol_error(error, error_cap, "address contains a non-hex character");
            return -1;
        }
        out[i] = (uint8_t)((hi << 4) | lo);
    }
    return 0;
}

static int atg_cobol_copy_result_bytes(
    atg_result *result,
    uint8_t *out,
    uint32_t cap,
    uint32_t *out_len,
    char *error,
    uint32_t error_cap
) {
    if (out_len) *out_len = 0;
    if (!result) {
        atg_cobol_error(error, error_cap, "runtime returned a null result");
        return -1;
    }
    const char *runtime_error = atg_result_error(result);
    if (runtime_error) {
        atg_cobol_error(error, error_cap, runtime_error);
        atg_result_free(result);
        return -1;
    }
    size_t n = atg_result_len(result);
    if (n > UINT32_MAX || n > cap || (n && !out)) {
        atg_cobol_error(error, error_cap, "output buffer is too small");
        atg_result_free(result);
        return -1;
    }
    if (n) memcpy(out, atg_result_data(result), n);
    if (out_len) *out_len = (uint32_t)n;
    atg_result_free(result);
    return 0;
}

/* Convert one unsigned big-endian EVM word to decimal without a bigint library. */
static int atg_cobol_u256_decimal(
    const uint8_t word[32],
    char *out,
    uint32_t cap,
    uint32_t *out_len,
    char *error,
    uint32_t error_cap
) {
    if (out_len) *out_len = 0;
    uint8_t value[32];
    char reverse[78];
    uint32_t digits = 0;
    memcpy(value, word, 32);

    int nonzero = 0;
    for (size_t i = 0; i < 32; ++i) nonzero |= value[i] != 0;
    if (!nonzero) {
        if (!out || cap < 1) {
            atg_cobol_error(error, error_cap, "decimal output buffer is too small");
            return -1;
        }
        out[0] = '0';
        if (out_len) *out_len = 1;
        return 0;
    }

    while (1) {
        unsigned carry = 0;
        nonzero = 0;
        for (size_t i = 0; i < 32; ++i) {
            unsigned current = (carry << 8) | value[i];
            value[i] = (uint8_t)(current / 10u);
            carry = current % 10u;
            nonzero |= value[i] != 0;
        }
        if (digits >= sizeof(reverse)) {
            atg_cobol_error(error, error_cap, "uint256 decimal conversion overflow");
            return -1;
        }
        reverse[digits++] = (char)('0' + carry);
        if (!nonzero) break;
    }

    if (!out || cap < digits) {
        atg_cobol_error(error, error_cap, "decimal output buffer is too small");
        return -1;
    }
    for (uint32_t i = 0; i < digits; ++i) out[i] = reverse[digits - i - 1];
    if (out_len) *out_len = digits;
    return 0;
}

static int atg_cobol_encode_address_u256_call(
    const char *abi,
    const char *signature,
    const char *owner_text,
    uint32_t owner_text_len,
    uint8_t *out,
    uint32_t out_cap,
    uint32_t *out_len,
    char *error,
    uint32_t error_cap
) {
    if (out_len) *out_len = 0;
    uint8_t address[20];
    uint8_t word[32] = {0};
    atg_value *args = NULL;
    atg_value *owner = NULL;
    atg_result *encoded = NULL;

    if (atg_cobol_parse_address(owner_text, owner_text_len, address, error, error_cap)) return -1;
    memcpy(word + 12, address, 20);

    args = atg_value_seq();
    owner = atg_value_word(word);
    if (!args || !owner || atg_value_push(args, owner)) {
        atg_value_free(owner);
        atg_value_free(args);
        atg_cobol_error(error, error_cap, "failed to construct ABI arguments");
        return -1;
    }
    atg_value_free(owner);
    encoded = atg_encode(abi, signature, args);
    atg_value_free(args);
    return atg_cobol_copy_result_bytes(encoded, out, out_cap, out_len, error, error_cap);
}

static int atg_cobol_decode_single_u256(
    const char *abi,
    const char *signature,
    const uint8_t *data,
    uint32_t data_len,
    char *decimal,
    uint32_t decimal_cap,
    uint32_t *decimal_len,
    char *error,
    uint32_t error_cap
) {
    if (decimal_len) *decimal_len = 0;
    atg_result *decoded = atg_decode(abi, signature, data, data_len);
    if (!decoded) {
        atg_cobol_error(error, error_cap, "runtime returned a null decode result");
        return -1;
    }
    const char *runtime_error = atg_result_error(decoded);
    if (runtime_error) {
        atg_cobol_error(error, error_cap, runtime_error);
        atg_result_free(decoded);
        return -1;
    }

    const atg_value *root = atg_result_value(decoded);
    if (!root || atg_value_kind(root) != 4 || atg_value_len(root) != 1) {
        atg_cobol_error(error, error_cap, "decoded result does not contain one value");
        atg_result_free(decoded);
        return -1;
    }
    const atg_value *value = atg_value_at(root, 0);
    if (!value || atg_value_kind(value) != 2) {
        atg_cobol_error(error, error_cap, "decoded result is not a uint256 word");
        atg_result_free(decoded);
        return -1;
    }
    uint8_t word[32];
    memcpy(word, atg_value_data(value), 32);
    atg_result_free(decoded);
    return atg_cobol_u256_decimal(word, decimal, decimal_cap, decimal_len, error, error_cap);
}

#ifdef ATG_COBOL_WITH_CURL
#define ATG_COBOL_MAX_HTTP_RESPONSE 65536u
struct atg_cobol_http_buffer {
    char *data;
    size_t len;
    size_t cap;
    int too_large;
};

static size_t atg_cobol_curl_write(char *ptr, size_t size, size_t nmemb, void *userdata) {
    struct atg_cobol_http_buffer *buffer = (struct atg_cobol_http_buffer *)userdata;
    if (size != 0 && nmemb > SIZE_MAX / size) return 0;
    size_t n = size * nmemb;
    if (n > ATG_COBOL_MAX_HTTP_RESPONSE - buffer->len) {
        buffer->too_large = 1;
        return 0;
    }
    if (buffer->len > SIZE_MAX - n - 1) return 0;
    size_t needed = buffer->len + n + 1;
    if (needed > buffer->cap) {
        size_t cap = buffer->cap ? buffer->cap : 512;
        while (cap < needed) {
            if (cap > SIZE_MAX / 2) return 0;
            cap *= 2;
        }
        char *next = (char *)realloc(buffer->data, cap);
        if (!next) return 0;
        buffer->data = next;
        buffer->cap = cap;
    }
    memcpy(buffer->data + buffer->len, ptr, n);
    buffer->len += n;
    buffer->data[buffer->len] = '\0';
    return n;
}

static int atg_cobol_hex_encode(const uint8_t *data, uint32_t len, char **out) {
    static const char alphabet[] = "0123456789abcdef";
    if (len > (UINT32_MAX - 3u) / 2u) return -1;
    size_t n = (size_t)len * 2u + 3u;
    char *hex = (char *)malloc(n);
    if (!hex) return -1;
    hex[0] = '0';
    hex[1] = 'x';
    for (uint32_t i = 0; i < len; ++i) {
        hex[2 + i * 2] = alphabet[data[i] >> 4];
        hex[3 + i * 2] = alphabet[data[i] & 15];
    }
    hex[2 + (size_t)len * 2] = '\0';
    *out = hex;
    return 0;
}

static int atg_cobol_json_result_hex(
    const char *json,
    size_t json_len,
    uint8_t *out,
    uint32_t out_cap,
    uint32_t *out_len,
    char *error,
    uint32_t error_cap
) {
    if (out_len) *out_len = 0;
    if (!json || json_len == 0 || json_len > ATG_COBOL_MAX_HTTP_RESPONSE) {
        atg_cobol_error(error, error_cap, "invalid JSON-RPC response length");
        return -1;
    }
    struct json_tokener *tokener = json_tokener_new();
    if (!tokener) {
        atg_cobol_error(error, error_cap, "JSON parser allocation failed");
        return -1;
    }
    json_tokener_set_flags(tokener, JSON_TOKENER_STRICT);
    struct json_object *root = json_tokener_parse_ex(tokener, json, (int)json_len);
    enum json_tokener_error parse_error = json_tokener_get_error(tokener);
    size_t parsed_len = json_tokener_get_parse_end(tokener);
    json_tokener_free(tokener);
    while (parsed_len < json_len && isspace((unsigned char)json[parsed_len])) ++parsed_len;
    if (parse_error != json_tokener_success || parsed_len != json_len ||
        !root || json_object_get_type(root) != json_type_object) {
        if (root) json_object_put(root);
        atg_cobol_error(error, error_cap, "invalid JSON-RPC response JSON");
        return -1;
    }
    struct json_object *version = NULL;
    struct json_object *id = NULL;
    struct json_object *result = NULL;
    struct json_object *rpc_error = NULL;
    if (!json_object_object_get_ex(root, "jsonrpc", &version) ||
        json_object_get_type(version) != json_type_string ||
        json_object_get_string_len(version) != 3 ||
        strcmp(json_object_get_string(version), "2.0") != 0 ||
        !json_object_object_get_ex(root, "id", &id) ||
        json_object_get_type(id) != json_type_int ||
        json_object_get_int64(id) != 1) {
        json_object_put(root);
        atg_cobol_error(error, error_cap, "invalid JSON-RPC version or id");
        return -1;
    }
    if (json_object_object_get_ex(root, "error", &rpc_error) &&
        json_object_get_type(rpc_error) != json_type_null) {
        json_object_put(root);
        atg_cobol_error(error, error_cap, "JSON-RPC returned an error");
        return -1;
    }
    if (!json_object_object_get_ex(root, "result", &result) ||
        json_object_get_type(result) != json_type_string) {
        json_object_put(root);
        atg_cobol_error(error, error_cap, "JSON-RPC result is not a hex string");
        return -1;
    }
    const char *p = json_object_get_string(result);
    size_t result_len = (size_t)json_object_get_string_len(result);
    if (result_len < 2 || p[0] != '0' || (p[1] != 'x' && p[1] != 'X')) {
        json_object_put(root);
        atg_cobol_error(error, error_cap, "JSON-RPC result lacks 0x prefix");
        return -1;
    }
    p += 2;
    size_t chars = result_len - 2;
    if (chars % 2 != 0 || chars / 2 > out_cap) {
        json_object_put(root);
        atg_cobol_error(error, error_cap, "JSON-RPC result is invalid or too large");
        return -1;
    }
    uint32_t bytes = (uint32_t)(chars / 2);
    for (uint32_t i = 0; i < bytes; ++i) {
        int hi = atg_cobol_hex_nibble((unsigned char)p[i * 2]);
        int lo = atg_cobol_hex_nibble((unsigned char)p[i * 2 + 1]);
        if (hi < 0 || lo < 0) {
            json_object_put(root);
            atg_cobol_error(error, error_cap, "JSON-RPC result contains non-hex data");
            return -1;
        }
        out[i] = (uint8_t)((hi << 4) | lo);
    }
    json_object_put(root);
    if (out_len) *out_len = bytes;
    return 0;
}
#endif

static int atg_cobol_eth_call(
    const char *rpc_url,
    uint32_t rpc_url_len,
    const char *contract_text,
    uint32_t contract_text_len,
    const uint8_t *calldata,
    uint32_t calldata_len,
    uint8_t *out,
    uint32_t out_cap,
    uint32_t *out_len,
    char *error,
    uint32_t error_cap
) {
    if (out_len) *out_len = 0;
#ifndef ATG_COBOL_WITH_CURL
    (void)rpc_url; (void)rpc_url_len; (void)contract_text; (void)contract_text_len;
    (void)calldata; (void)calldata_len; (void)out; (void)out_cap; (void)out_len;
    atg_cobol_error(error, error_cap, "RPC bridge was built without ATG_COBOL_WITH_CURL");
    return -1;
#else
    rpc_url_len = atg_cobol_trim_len(rpc_url, rpc_url_len);
    contract_text_len = atg_cobol_trim_len(contract_text, contract_text_len);
    if (!rpc_url || rpc_url_len == 0 || rpc_url_len > 4096) {
        atg_cobol_error(error, error_cap, "invalid RPC URL");
        return -1;
    }
    uint8_t address[20];
    if (atg_cobol_parse_address(contract_text, contract_text_len, address, error, error_cap)) return -1;

    char contract_hex[43];
    static const char alphabet[] = "0123456789abcdef";
    contract_hex[0] = '0'; contract_hex[1] = 'x';
    for (size_t i = 0; i < 20; ++i) {
        contract_hex[2 + i * 2] = alphabet[address[i] >> 4];
        contract_hex[3 + i * 2] = alphabet[address[i] & 15];
    }
    contract_hex[42] = '\0';

    char *data_hex = NULL;
    if (atg_cobol_hex_encode(calldata, calldata_len, &data_hex)) {
        atg_cobol_error(error, error_cap, "failed to encode calldata hex");
        return -1;
    }

    size_t body_cap = strlen(data_hex) + sizeof(contract_hex) + 160;
    char *body = (char *)malloc(body_cap);
    char *url = (char *)malloc((size_t)rpc_url_len + 1);
    if (!body || !url) {
        free(data_hex); free(body); free(url);
        atg_cobol_error(error, error_cap, "allocation failure");
        return -1;
    }
    memcpy(url, rpc_url, rpc_url_len);
    url[rpc_url_len] = '\0';
    snprintf(
        body,
        body_cap,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_call\",\"params\":[{\"to\":\"%s\",\"data\":\"%s\"},\"latest\"]}",
        contract_hex,
        data_hex
    );
    free(data_hex);

    CURL *curl = curl_easy_init();
    if (!curl) {
        free(body); free(url);
        atg_cobol_error(error, error_cap, "curl initialization failed");
        return -1;
    }
    struct atg_cobol_http_buffer response = {0};
    struct curl_slist *headers = NULL;
    headers = curl_slist_append(headers, "Content-Type: application/json");
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long)strlen(body));
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, atg_cobol_curl_write);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT_MS, 10000L);

    CURLcode code = curl_easy_perform(curl);
    long http_status = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &http_status);
    curl_slist_free_all(headers);
    curl_easy_cleanup(curl);
    free(body);
    free(url);

    if (code != CURLE_OK || http_status < 200 || http_status >= 300 || !response.data) {
        free(response.data);
        atg_cobol_error(error, error_cap, response.too_large ? "RPC response exceeds 65536 bytes" :
            (code != CURLE_OK ? curl_easy_strerror(code) : "RPC HTTP request failed"));
        return -1;
    }
    int status = atg_cobol_json_result_hex(response.data, response.len, out, out_cap, out_len, error, error_cap);
    free(response.data);
    return status;
#endif
}
"#;

fn render_function(out: &mut String, abi_name: &str, f: &PocFunction) {
    let signature = c_literal(&f.signature);
    out.push_str(&format!(
        r#"
static const char {stem}_signature[] = {signature};

int {stem}_encode(
    const char *owner, uint32_t owner_len,
    uint8_t *out, uint32_t out_cap, uint32_t *out_len,
    char *error, uint32_t error_cap
) {{
    if (out_len) *out_len = 0;
    return atg_cobol_encode_address_u256_call(
        {abi_name}, {stem}_signature,
        owner, owner_len,
        out, out_cap, out_len,
        error, error_cap
    );
}}

int {stem}_decode(
    const uint8_t *data, uint32_t data_len,
    char *decimal, uint32_t decimal_cap, uint32_t *decimal_len,
    char *error, uint32_t error_cap
) {{
    if (decimal_len) *decimal_len = 0;
    if (data_len > 4096u) {{
        atg_cobol_error(error, error_cap, "ABI result length exceeds 4096 bytes");
        return -1;
    }}
    return atg_cobol_decode_single_u256(
        {abi_name}, {stem}_signature,
        data, data_len,
        decimal, decimal_cap, decimal_len,
        error, error_cap
    );
}}

int {stem}_call(
    const char *rpc_url, uint32_t rpc_url_len,
    const char *contract_address, uint32_t contract_address_len,
    const char *owner, uint32_t owner_len,
    char *decimal, uint32_t decimal_cap, uint32_t *decimal_len,
    char *error, uint32_t error_cap
) {{
    if (decimal_len) *decimal_len = 0;
    uint8_t calldata[256];
    uint32_t calldata_len = 0;
    uint8_t response[4096];
    uint32_t response_len = 0;
    if ({stem}_encode(
            owner, owner_len,
            calldata, (uint32_t)sizeof(calldata), &calldata_len,
            error, error_cap)) return -1;
    if (atg_cobol_eth_call(
            rpc_url, rpc_url_len,
            contract_address, contract_address_len,
            calldata, calldata_len,
            response, (uint32_t)sizeof(response), &response_len,
            error, error_cap)) return -1;
    return {stem}_decode(
        response, response_len,
        decimal, decimal_cap, decimal_len,
        error, error_cap
    );
}}
"#,
        stem = f.bridge_stem,
    ));
}

pub(super) fn render_bridge(ir: &ContractIr, functions: &[PocFunction], wrappers: bool) -> String {
    let prefix = c_ident(&ir.name);
    let abi_name = format!("atg_cobol_{prefix}_abi");
    let abi = c_literal(&serde_json::to_string(&ir.raw_abi).expect("ABI serializes"));

    let mut out = format!(
        "/* Generated by abi-typegen. Do not edit. */\n/* COBOL bridge for `{}`. */\n",
        ir.name
    );
    if wrappers && !functions.is_empty() {
        out.push_str(SUPPORT);
    }
    out.push_str(&format!("\nconst char {abi_name}[] = {abi};\n"));

    if wrappers {
        for f in functions {
            render_function(&mut out, &abi_name, f);
        }
    }
    out
}
