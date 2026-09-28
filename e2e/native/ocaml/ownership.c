/* Inject an OCaml exception at the result-copy boundary and count native frees. */
#include <caml/mlvalues.h>
#include <caml/alloc.h>
#include <caml/fail.h>
#include "abi_typegen.h"
static int fail_copy = 0;
static int releases = 0;
static void counted_result_free(atg_result *r) {
  if (r) ++releases;
  atg_result_free(r);
}
static value checked_string(mlsize_t length, const char *data) {
  if (fail_copy) { fail_copy = 0; caml_raise_out_of_memory(); }
  return caml_alloc_initialized_string(length, data);
}
static value checked_copy(const char *data) {
  if (fail_copy) { fail_copy = 0; caml_raise_out_of_memory(); }
  return caml_copy_string(data);
}
#define atg_result_free counted_result_free
#define caml_alloc_initialized_string checked_string
#define caml_copy_string checked_copy
#include "Atg_sample.ocaml.c"
CAMLprim value test_fail_next_copy(value unit) {
  (void)unit; fail_copy = 1; return Val_unit;
}
CAMLprim value test_release_count(value unit) {
  (void)unit; return Val_int(releases);
}
