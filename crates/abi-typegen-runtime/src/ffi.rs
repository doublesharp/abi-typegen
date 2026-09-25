//! C ABI. See `include/abi_typegen.h` for pointer ownership and lifetime rules.
use crate::codec::{self, Value};
use alloy_primitives::B256;
use std::{
    alloc::{Layout, alloc_zeroed, dealloc},
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr, slice,
};

/// Opaque owned ABI value. Child pointers are borrowed from their parent.
pub struct AtgValue {
    _private: [u8; 0],
}
/// Opaque result owning its bytes, decoded value, diagnostics, and scratch allocations.
pub struct AtgResult {
    bytes: Vec<u8>,
    value: Option<Value>,
    error: Option<CString>,
    allocations: Allocations,
}
#[derive(Default)]
struct Allocations(Vec<(*mut u8, Layout)>);
impl Drop for Allocations {
    fn drop(&mut self) {
        for (pointer, layout) in self.0.drain(..) {
            // SAFETY: Each allocation was created with this layout and is freed once.
            unsafe { dealloc(pointer, layout) };
        }
    }
}
impl AtgResult {
    fn empty() -> Self {
        Self {
            bytes: vec![],
            value: None,
            error: None,
            allocations: Allocations::default(),
        }
    }
    fn failure(message: impl ToString) -> Self {
        Self {
            error: Some(CString::new(message.to_string().replace('\0', " ")).expect("NUL removed")),
            ..Self::empty()
        }
    }
}
fn guarded<T: Default>(f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_default()
}
fn result(f: impl FnOnce() -> Result<AtgResult, String>) -> *mut AtgResult {
    let result = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => AtgResult::failure(error),
        Err(_) => AtgResult::failure("ABI runtime panic"),
    };
    Box::into_raw(Box::new(result))
}
// SAFETY: Callers guarantee readable storage for non-null pointers for the stated lengths.
unsafe fn bytes<'a>(data: *const u8, len: usize) -> Result<&'a [u8], String> {
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() || len > isize::MAX as usize {
        return Err("invalid byte buffer".into());
    }
    // SAFETY: Guaranteed by this function's caller, with size checked above.
    Ok(unsafe { slice::from_raw_parts(data, len) })
}
unsafe fn text<'a>(data: *const c_char) -> Result<&'a str, String> {
    if data.is_null() {
        return Err("null string".into());
    }
    // SAFETY: C entrypoints require a valid NUL-terminated string.
    unsafe { CStr::from_ptr(data) }
        .to_str()
        .map_err(|e| e.to_string())
}
unsafe fn value<'a>(data: *const AtgValue) -> Option<&'a Value> {
    // SAFETY: AtgValue is opaque; pointers denote Value allocations or borrowed Value children.
    unsafe { (data as *const Value).as_ref() }
}
fn boxed(value: Value) -> *mut AtgValue {
    Box::into_raw(Box::new(value)) as *mut AtgValue
}

/// Create a boolean value.
#[unsafe(no_mangle)]
pub extern "C" fn atg_value_bool(v: i32) -> *mut AtgValue {
    guarded(|| boxed(Value::Bool(v != 0)))
}
/// Copy a 32-byte big-endian integer/address word.
/// # Safety
/// `data` must reference 32 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_word(data: *const u8) -> *mut AtgValue {
    guarded(|| {
        // SAFETY: Required by this entrypoint's contract.
        unsafe { bytes(data, 32) }
            .map(|b| boxed(Value::Word(B256::from_slice(b))))
            .unwrap_or(ptr::null_mut())
    })
}
/// Copy a byte/string value.
/// # Safety
/// `data` must reference `len` readable bytes, or may be null when len is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_bytes(data: *const u8, len: usize) -> *mut AtgValue {
    // SAFETY: Required by this entrypoint's contract.
    guarded(|| {
        unsafe { bytes(data, len) }
            .map(|b| boxed(Value::Bytes(b.to_vec())))
            .unwrap_or(ptr::null_mut())
    })
}
/// Create an empty sequence.
#[unsafe(no_mangle)]
pub extern "C" fn atg_value_seq() -> *mut AtgValue {
    guarded(|| boxed(Value::Seq(vec![])))
}
/// Clone a child into a sequence. Returns zero on success.
/// # Safety
/// Pointers must be live runtime values; seq must be owned and exclusively mutable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_push(seq: *mut AtgValue, child: *const AtgValue) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Required by this entrypoint's contract. Clone before taking mutable reference.
        let Some(child) = (unsafe { value(child) }).cloned() else {
            return -1;
        };
        // SAFETY: seq is exclusively mutable and was created by boxed.
        let Some(Value::Seq(xs)) = (unsafe { (seq as *mut Value).as_mut() }) else {
            return -1;
        };
        xs.push(child);
        0
    }))
    .unwrap_or(-1)
}
/// Free an owned value; accepts null.
/// # Safety
/// Pointer must be owned, live, and returned by a value constructor, never a borrowed child.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_free(v: *mut AtgValue) {
    if !v.is_null() {
        guarded(|| {
            // SAFETY: Constructor allocated this Value and caller transfers ownership once.
            drop(unsafe { Box::from_raw(v as *mut Value) });
        });
    }
}
/// Return the value kind, or zero for null.
/// # Safety
/// Pointer must be null or a live runtime value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_kind(v: *const AtgValue) -> i32 {
    // SAFETY: Required by entrypoint contract.
    match unsafe { value(v) } {
        Some(Value::Bool(_)) => 1,
        Some(Value::Word(_)) => 2,
        Some(Value::Bytes(_)) => 3,
        Some(Value::Seq(_)) => 4,
        None => 0,
    }
}
/// Read a boolean; returns zero for null or another kind.
/// # Safety
/// Pointer must be null or a live runtime value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_get_bool(v: *const AtgValue) -> i32 {
    // SAFETY: Required by entrypoint contract.
    i32::from(matches!(unsafe { value(v) }, Some(Value::Bool(true))))
}
/// Borrow word/byte storage until the value is mutated or freed.
/// # Safety
/// Pointer must be null or a live runtime value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_data(v: *const AtgValue) -> *const u8 {
    // SAFETY: Required by entrypoint contract.
    match unsafe { value(v) } {
        Some(Value::Word(w)) => w.as_ptr(),
        Some(Value::Bytes(b)) => b.as_ptr(),
        _ => ptr::null(),
    }
}
/// Return byte count or sequence length.
/// # Safety
/// Pointer must be null or a live runtime value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_len(v: *const AtgValue) -> usize {
    // SAFETY: Required by entrypoint contract.
    match unsafe { value(v) } {
        Some(Value::Word(_)) => 32,
        Some(Value::Bytes(b)) => b.len(),
        Some(Value::Seq(xs)) => xs.len(),
        _ => 0,
    }
}
/// Borrow a child, or return null when index is out of bounds.
/// # Safety
/// Pointer must be null or a live runtime value. Do not free or mutate the returned child.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_value_at(v: *const AtgValue, index: usize) -> *const AtgValue {
    // SAFETY: Required by entrypoint contract.
    match unsafe { value(v) } {
        Some(Value::Seq(xs)) => xs
            .get(index)
            .map_or(ptr::null(), |v| v as *const Value as *const AtgValue),
        _ => ptr::null(),
    }
}
/// Encode a canonical function signature and positional sequence.
/// # Safety
/// Strings must be NUL-terminated UTF-8; args must be a live value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_encode(
    abi: *const c_char,
    signature: *const c_char,
    args: *const AtgValue,
) -> *mut AtgResult {
    result(|| {
        // SAFETY: Required by entrypoint contract.
        let (abi, signature, args) = unsafe { (text(abi)?, text(signature)?, value(args)) };
        let Some(Value::Seq(args)) = args else {
            return Err("arguments must be a sequence".into());
        };
        Ok(AtgResult {
            bytes: codec::encode(abi, signature, args).map_err(|e| e.to_string())?,
            ..AtgResult::empty()
        })
    })
}
macro_rules! decoder {
    ($name:ident,$implementation:ident) => {
        /// Decode bytes into a positional value sequence using a canonical signature.
        /// # Safety
        /// Strings must be NUL-terminated UTF-8 and data must reference len readable bytes.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            abi: *const c_char,
            signature: *const c_char,
            data: *const u8,
            len: usize,
        ) -> *mut AtgResult {
            result(|| {
                // SAFETY: Required by entrypoint contract.
                let (abi, signature, data) =
                    unsafe { (text(abi)?, text(signature)?, bytes(data, len)?) };
                Ok(AtgResult {
                    value: Some(
                        codec::$implementation(abi, signature, data).map_err(|e| e.to_string())?,
                    ),
                    ..AtgResult::empty()
                })
            })
        }
    };
}
decoder!(atg_decode, decode);
decoder!(atg_decode_error, decode_error);
/// Decode event fields in declaration order, preserving hashed indexed fields as bytes32.
/// # Safety
/// Strings must be valid C strings; topics references count*32 bytes; data references len bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_decode_event(
    abi: *const c_char,
    signature: *const c_char,
    topics: *const u8,
    count: usize,
    data: *const u8,
    len: usize,
) -> *mut AtgResult {
    result(|| {
        if count > 4 {
            return Err("event has more than four topics".into());
        }
        // SAFETY: Required by entrypoint contract; count is bounded.
        let (abi, signature, topics, data) = unsafe {
            (
                text(abi)?,
                text(signature)?,
                bytes(topics, count * 32)?,
                bytes(data, len)?,
            )
        };
        let topics: Vec<_> = topics.chunks_exact(32).map(B256::from_slice).collect();
        Ok(AtgResult {
            value: Some(
                codec::decode_event(abi, signature, &topics, data).map_err(|e| e.to_string())?,
            ),
            ..AtgResult::empty()
        })
    })
}
/// Borrow the diagnostic string; null means success. A null result reports failure.
/// # Safety
/// Pointer must be null or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_error(r: *const AtgResult) -> *const c_char {
    // SAFETY: Required by entrypoint contract.
    unsafe { r.as_ref() }.map_or(c"null runtime result".as_ptr(), |r| {
        r.error.as_ref().map_or(ptr::null(), |e| e.as_ptr())
    })
}
/// Borrow encoded bytes until the result is freed.
/// # Safety
/// Pointer must be null or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_data(r: *const AtgResult) -> *const u8 {
    // SAFETY: Required by entrypoint contract.
    unsafe { r.as_ref() }.map_or(ptr::null(), |r| r.bytes.as_ptr())
}
/// Return encoded byte count.
/// # Safety
/// Pointer must be null or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_len(r: *const AtgResult) -> usize {
    // SAFETY: Required by entrypoint contract.
    unsafe { r.as_ref() }.map_or(0, |r| r.bytes.len())
}
/// Borrow the decoded sequence until the result is freed.
/// # Safety
/// Pointer must be null or a live result; do not free returned pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_value(r: *const AtgResult) -> *const AtgValue {
    // SAFETY: Required by entrypoint contract.
    unsafe { r.as_ref() }
        .and_then(|r| r.value.as_ref())
        .map_or(ptr::null(), |v| v as *const Value as *const AtgValue)
}
/// Allocate aligned zeroed scratch storage owned by result.
/// # Safety
/// Result must be live and exclusively mutable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_alloc(
    r: *mut AtgResult,
    size: usize,
    alignment: usize,
) -> *mut c_void {
    guarded(|| {
        // SAFETY: Required by entrypoint contract.
        let Some(r) = (unsafe { r.as_mut() }) else {
            return ptr::null_mut();
        };
        let Ok(layout) = Layout::from_size_align(size.max(1), alignment) else {
            return ptr::null_mut();
        };
        // SAFETY: Layout is valid and nonempty.
        let pointer = unsafe { alloc_zeroed(layout) };
        if !pointer.is_null() {
            r.allocations.0.push((pointer, layout));
        }
        pointer.cast()
    })
}
/// Free a result and all its borrowed storage; accepts null.
/// # Safety
/// Result must be live and owned, and must not be freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_free(r: *mut AtgResult) {
    if !r.is_null() {
        guarded(|| {
            // SAFETY: Caller transfers a live uniquely owned Box returned by result().
            drop(unsafe { Box::from_raw(r) });
        });
    }
}
/// Copy transport response bytes into an owned result.
/// # Safety
/// Data must reference len readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_bytes(data: *const u8, len: usize) -> *mut AtgResult {
    result(|| {
        // SAFETY: Required by entrypoint contract.
        Ok(AtgResult {
            bytes: unsafe { bytes(data, len)? }.to_vec(),
            ..AtgResult::empty()
        })
    })
}
/// Copy a transport failure message into an owned result.
/// # Safety
/// Message must be a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_result_failure(message: *const c_char) -> *mut AtgResult {
    // SAFETY: Required by entrypoint contract.
    result(|| Err(unsafe { text(message)? }.to_string()))
}

/// Append typed constructor arguments to caller-supplied deployment bytecode.
/// # Safety
/// ABI must be a NUL-terminated UTF-8 string, bytecode must reference len bytes,
/// and args must be a live sequence value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atg_encode_constructor(
    abi: *const c_char,
    bytecode: *const u8,
    len: usize,
    args: *const AtgValue,
) -> *mut AtgResult {
    result(|| {
        // SAFETY: Required by this entrypoint's contract.
        let (abi, bytecode, args) = unsafe { (text(abi)?, bytes(bytecode, len)?, value(args)) };
        let Some(Value::Seq(args)) = args else {
            return Err("constructor arguments must be a sequence".into());
        };
        Ok(AtgResult {
            bytes: codec::encode_constructor(abi, bytecode, args).map_err(|e| e.to_string())?,
            ..AtgResult::empty()
        })
    })
}
