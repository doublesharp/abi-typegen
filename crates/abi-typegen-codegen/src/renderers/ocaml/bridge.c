#include <caml/mlvalues.h>
#include <caml/memory.h>
#include <caml/alloc.h>
#include <caml/fail.h>
#include <caml/custom.h>
#include <string.h>
#include "abi_typegen.h"
/* A rooted custom block owns the native result while OCaml allocations can raise.
 * On success it is emptied immediately; after an exception its finalizer frees
 * the result once the unreachable owner is collected. Allocate it before the
 * runtime call so failure to allocate the owner cannot leak a native result. */
static void release_result(value owner) {
  atg_result **slot=(atg_result **)Data_custom_val(owner);
  if (*slot) { atg_result_free(*slot); *slot=NULL; }
}
static struct custom_operations result_operations = {
  .identifier="abi-typegen.ocaml.result.v1",
  .finalize=release_result,
  .compare=custom_compare_default,
  .hash=custom_hash_default,
  .serialize=custom_serialize_default,
  .deserialize=custom_deserialize_default,
  .compare_ext=custom_compare_ext_default,
  .fixed_length=custom_fixed_length_default
};
static atg_value *to_value(value v) {
  atg_value *r;
  switch (Tag_val(v)) {
    case 0: return atg_value_bool(Bool_val(Field(v,0)));
    case 1: if (caml_string_length(Field(v,0)) != 32) return NULL; return atg_value_word((const uint8_t *)String_val(Field(v,0)));
    case 2: return atg_value_bytes((const uint8_t *)String_val(Field(v,0)),caml_string_length(Field(v,0)));
    case 3:
      r=atg_value_seq(); if (!r) return NULL;
      for (mlsize_t i=0;i<Wosize_val(Field(v,0));i++) {
        atg_value *child=to_value(Field(Field(v,0),i));
        if (!child) { atg_value_free(r); return NULL; }
        int failed=atg_value_push(r,child); atg_value_free(child);
        if (failed) { atg_value_free(r); return NULL; }
      } return r;
    default: return NULL;
  }
}
static value from_value(const atg_value *v) {
  CAMLparam0(); CAMLlocal3(r,payload,child);
  int kind=atg_value_kind(v);
  if (kind==1) payload=Val_bool(atg_value_get_bool(v));
  else if (kind==2 || kind==3) payload=caml_alloc_initialized_string(kind==2 ? 32 : atg_value_len(v),(const char *)atg_value_data(v));
  else if (kind==4) {
    size_t n=atg_value_len(v); payload=caml_alloc(n,0);
    for (size_t i=0;i<n;i++) Store_field(payload,i,Val_unit);
    for (size_t i=0;i<n;i++) { child=from_value(atg_value_at(v,i)); Store_field(payload,i,child); }
  } else caml_failwith("invalid runtime value");
  r=caml_alloc(1,kind-1); Store_field(r,0,payload); CAMLreturn(r);
}
/* Request: operation, ABI, signature, values, data, topics. */
static value request_impl(value request) {
  CAMLparam1(request); CAMLlocal3(output,error,owner);
  owner=caml_alloc_custom(&result_operations,sizeof(atg_result *),0,1);
  *((atg_result **)Data_custom_val(owner))=NULL;
  atg_result *r=NULL;
  int op=Int_val(Field(request,0));
  const char *abi=String_val(Field(request,1));
  const char *signature=String_val(Field(request,2));
  const uint8_t *data=(const uint8_t *)String_val(Field(request,4));
  size_t len=caml_string_length(Field(request,4));
  if (op==0 || op==4) {
    atg_value *args=to_value(Field(request,3));
    if (!args) caml_failwith("ABI value allocation failed");
    r=op==0 ? atg_encode(abi,signature,args) : atg_encode_constructor(abi,data,len,args);
    atg_value_free(args);
  } else if (op==1) r=atg_decode(abi,signature,data,len);
  else if (op==2) r=atg_decode_error(abi,signature,data,len);
  else if (op==3) {
    value topics=Field(request,5);
    size_t size=caml_string_length(topics);
    if (size%32) caml_invalid_argument("topics must contain 32-byte words");
    r=atg_decode_event(abi,signature,(const uint8_t *)String_val(topics),size/32,data,len);
  }
  *((atg_result **)Data_custom_val(owner))=r;
  if (!r) caml_failwith("ABI runtime allocation failed");
  const char *message=atg_result_error(r);
  if (message) { error=caml_copy_string(message); release_result(owner); caml_failwith_value(error); }
  if (op==0 || op==4) { output=caml_alloc_initialized_string(atg_result_len(r),(const char *)atg_result_data(r)); }
  else output=from_value(atg_result_value(r));
  release_result(owner); CAMLreturn(output);
}

CAMLprim value ATG_PREFIX_encode(value request) {
  int op=Int_val(Field(request,0));
  if (op!=0 && op!=4) caml_invalid_argument("invalid encode operation");
  return request_impl(request);
}
CAMLprim value ATG_PREFIX_decode(value request) {
  int op=Int_val(Field(request,0));
  if (op<1 || op>3) caml_invalid_argument("invalid decode operation");
  return request_impl(request);
}
