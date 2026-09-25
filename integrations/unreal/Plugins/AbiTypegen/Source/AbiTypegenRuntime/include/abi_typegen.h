#ifndef ABI_TYPEGEN_RUNTIME_H
#define ABI_TYPEGEN_RUNTIME_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Runtime ABI version. Generated bindings require version 1. */
#define ATG_ABI_VERSION 1
/* Opaque owned values. Child values returned by atg_value_at are borrowed. */
typedef struct AtgValue atg_value;
typedef struct AtgResult atg_result;
typedef struct { uint8_t bytes[32]; } atg_word;
typedef struct { uint8_t bytes[20]; } atg_address;
typedef struct { const uint8_t *data; size_t len; } atg_bytes;
/* All pointers must reference valid storage for their documented length.
 * NULL is accepted for an empty byte buffer. Strings are UTF-8.
 * Creation failure returns NULL. Never free borrowed child values. */
atg_value *atg_value_bool(int value);
atg_value *atg_value_word(const uint8_t *word32);
atg_value *atg_value_bytes(const uint8_t *data, size_t len);
atg_value *atg_value_seq(void);
/* Push clones the child; caller retains ownership. Returns 0 on success. */
int atg_value_push(atg_value *seq, const atg_value *child);
void atg_value_free(atg_value *value);
/* Kind: 1 bool, 2 word, 3 bytes/string, 4 sequence. */
int atg_value_kind(const atg_value *value);
int atg_value_get_bool(const atg_value *value);
const uint8_t *atg_value_data(const atg_value *value);
size_t atg_value_len(const atg_value *value);
const atg_value *atg_value_at(const atg_value *value, size_t index);
/* ABI and signature are NUL-terminated UTF-8. args is a sequence.
 * Every non-NULL result must be released, on success or failure. */
atg_result *atg_encode(const char *abi, const char *signature, const atg_value *args);
atg_result *atg_encode_constructor(const char *abi, const uint8_t *bytecode, size_t len, const atg_value *args);
atg_result *atg_decode(const char *abi, const char *signature, const uint8_t *data, size_t len);
atg_result *atg_decode_error(const char *abi, const char *signature, const uint8_t *data, size_t len);
/* Topics are contiguous 32-byte words, including topic0 for non-anonymous events. */
atg_result *atg_decode_event(const char *abi, const char *signature, const uint8_t *topics, size_t count, const uint8_t *data, size_t len);
/* NULL means success. The error string is borrowed until result_free. */
const char *atg_result_error(const atg_result *result);
const uint8_t *atg_result_data(const atg_result *result);
size_t atg_result_len(const atg_result *result);
const atg_value *atg_result_value(const atg_result *result);
/* Zeroed aligned scratch storage owned by result, for decoded C arrays. */
void *atg_result_alloc(atg_result *result, size_t size, size_t alignment);
void atg_result_free(atg_result *result);
/* Construct a result by copying bytes or an error from a transport adapter. */
atg_result *atg_result_bytes(const uint8_t *data, size_t len);
atg_result *atg_result_failure(const char *message);
/* Optional transaction fields. NULL pointers mean provider defaults. Numeric
 * words are unsigned, big-endian. A nonpayable wrapper rejects nonzero value. */
typedef struct {
    const atg_word *value;
    const uint64_t *gas_limit;
    const uint64_t *nonce;
    const atg_word *gas_price;
    const atg_word *max_fee_per_gas;
    const atg_word *max_priority_fee_per_gas;
    const char *block;
} atg_call_options;
/* Topic positions with has_topic=0 are wildcards. */
typedef struct {
    atg_address address;
    atg_word topics[4];
    uint8_t has_topic[4];
    size_t topic_count;
} atg_log_filter;
/* Caller-owned transport context. The callback returns a runtime-owned result.
 * write=0: eth_call result bytes; write=1: submitted transaction hash bytes.
 * write=2: deployment transaction, address is NULL, data is creation bytecode.
 * options contains transaction/block options and may be NULL. The callback
 * owns networking, signing, nonce management, and receipt policy. */
typedef atg_result *(*atg_transport_fn)(void *context, const atg_address *address, const uint8_t *data, size_t len, int write, const atg_call_options *options);
#ifdef __cplusplus
}
#endif
#endif
