/* q C extension. Build against the official KX k.h and abi_typegen.h.
 * macOS: cc -shared -fPIC -undefined dynamic_lookup -I/path/to/k/include
 *   -I. abi_typegen_q.c -L/path/to/runtime -labi_typegen_runtime -o abi_typegen_q.so
 * Linux: cc -shared -fPIC -I/path/to/k/include -I. abi_typegen_q.c
 *   -L/path/to/runtime -labi_typegen_runtime -o abi_typegen_q.so
 * Configure the platform library search path for libabi_typegen_runtime.
 * q: .atgContract.loadBridge[`:./abi_typegen_q]
 * Arguments and borrowed runtime values are never retained. */
#define KXVER 3
#include "k.h"
#include "abi_typegen.h"
#include <limits.h>
#include <stdlib.h>
#include <string.h>

static K atg_q_value(const atg_value *value, unsigned depth) {
    if (!value || depth > 64) return NULL;
    int kind = atg_value_kind(value);
    if (kind == 1) return kb(atg_value_get_bool(value));
    size_t len = atg_value_len(value);
    if (kind == 2) len = 32;
    if (len > (size_t)LLONG_MAX) return NULL;
    if (kind == 2 || kind == 3) {
        K out = ktn(KG, (J)len);
        if (!out) return NULL;
        if (len) memcpy(kG(out), atg_value_data(value), len);
        return out;
    }
    if (kind == 4) {
        K out = ktn(0, (J)len);
        if (!out) return NULL;
        /* Initialize before recursion so partial failure can be released safely. */
        for (size_t i = 0; i < len; ++i) kK(out)[i] = NULL;
        for (size_t i = 0; i < len; ++i) {
            kK(out)[i] = atg_q_value(atg_value_at(value, i), depth + 1);
            if (!kK(out)[i]) {
                /* Only release initialized children; q need not accept NULL slots. */
                for (size_t j = 0; j < i; ++j) r0(kK(out)[j]);
                out->n = 0;
                r0(out);
                return NULL;
            }
        }
        return out;
    }
    return NULL;
}

static char *atg_q_string(K value) {
    if (!value || value->t != KC || value->n < 0 ||
        (UJ)value->n >= SIZE_MAX || memchr(kC(value), 0, (size_t)value->n)) return NULL;
    char *out = malloc((size_t)value->n + 1);
    if (!out) return NULL;
    memcpy(out, kC(value), (size_t)value->n);
    out[value->n] = 0;
    return out;
}

/* Export for q's 2: loader: one general list (ABI;signature;topics;data).
 * topics is a flat byte vector of contiguous 32-byte topics, including topic0
 * only for non-anonymous events. data is a byte vector. */
#if defined(_WIN32)
__declspec(dllexport)
#endif
K atg_q_decode(K args) {
    if (!args || args->t != 0 || args->n != 4) return krr("expected (abi;signature;topics;data)");
    K topics = kK(args)[2], data = kK(args)[3];
    if (!topics || topics->t != KG || topics->n < 0 || topics->n > 128 || topics->n % 32 ||
        !data || data->t != KG || data->n < 0 || (UJ)data->n > SIZE_MAX)
        return krr("topics and data must be byte vectors; topics must contain at most four words");
    char *abi = atg_q_string(kK(args)[0]);
    char *signature = atg_q_string(kK(args)[1]);
    if (!abi || !signature) {
        free(abi); free(signature);
        return krr("ABI and signature must be NUL-free character vectors");
    }
    atg_result *result = atg_decode_event(abi, signature, kG(topics), (size_t)topics->n / 32,
                                         kG(data), (size_t)data->n);
    free(abi); free(signature);
    if (!result) return krr("ABI runtime allocation failure");
    const char *error = atg_result_error(result);
    if (error) {
        /* krr retains its pointer; ss gives it q-owned interned storage. */
        K failure = krr(ss((S)error));
        atg_result_free(result);
        return failure;
    }
    K out = atg_q_value(atg_result_value(result), 0);
    atg_result_free(result);
    return out ? out : krr("ABI value conversion failed or exceeds nesting limit");
}
