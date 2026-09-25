use alloy_dyn_abi::{DynSolType, DynSolValue, EventExt, FunctionExt, JsonAbiExt, Specifier};
use alloy_json_abi::JsonAbi;
use alloy_primitives::{Address, B256, I256, U256};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Bool(bool),
    Word(B256),
    Bytes(Vec<u8>),
    Seq(Vec<Value>),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("{0}")]
    Invalid(String),
}
type Result<T> = std::result::Result<T, Error>;
fn err(message: impl ToString) -> Error {
    Error::Invalid(message.to_string())
}

fn parse(abi: &str) -> Result<JsonAbi> {
    serde_json::from_str(abi).map_err(err)
}

fn convert(ty: &DynSolType, value: &Value, depth: usize) -> Result<DynSolValue> {
    if depth > 64 {
        return Err(err("ABI nesting exceeds 64"));
    }
    Ok(match (ty, value) {
        (DynSolType::Bool, Value::Bool(x)) => DynSolValue::Bool(*x),
        (DynSolType::Uint(bits), Value::Word(x)) => {
            let n = U256::from_be_bytes(x.0);
            if n.bit_len() > *bits {
                return Err(err("unsigned integer out of range"));
            }
            DynSolValue::Uint(n, *bits)
        }
        (DynSolType::Int(bits), Value::Word(x)) => {
            let n = I256::from_raw(U256::from_be_bytes(x.0));
            if *bits < 256
                && (n < -(I256::ONE << (*bits - 1)) || n > (I256::ONE << (*bits - 1)) - I256::ONE)
            {
                return Err(err("signed integer out of range"));
            }
            DynSolValue::Int(n, *bits)
        }
        (DynSolType::Address, Value::Word(x)) => {
            if x[..12].iter().any(|b| *b != 0) {
                return Err(err("address exceeds 20 bytes"));
            }
            DynSolValue::Address(Address::from_slice(&x[12..]))
        }
        (DynSolType::FixedBytes(n), Value::Bytes(x)) if x.len() == *n => {
            let mut word = B256::ZERO;
            word[..*n].copy_from_slice(x);
            DynSolValue::FixedBytes(word, *n)
        }
        (DynSolType::Bytes, Value::Bytes(x)) => DynSolValue::Bytes(x.clone()),
        (DynSolType::String, Value::Bytes(x)) => {
            DynSolValue::String(String::from_utf8(x.clone()).map_err(err)?)
        }
        (DynSolType::Array(inner), Value::Seq(xs)) => DynSolValue::Array(
            xs.iter()
                .map(|x| convert(inner, x, depth + 1))
                .collect::<Result<_>>()?,
        ),
        (DynSolType::FixedArray(inner, n), Value::Seq(xs)) if xs.len() == *n => {
            DynSolValue::FixedArray(
                xs.iter()
                    .map(|x| convert(inner, x, depth + 1))
                    .collect::<Result<_>>()?,
            )
        }
        (DynSolType::Tuple(types), Value::Seq(xs)) if xs.len() == types.len() => {
            DynSolValue::Tuple(
                types
                    .iter()
                    .zip(xs)
                    .map(|(t, x)| convert(t, x, depth + 1))
                    .collect::<Result<_>>()?,
            )
        }
        _ => return Err(err(format!("value does not match {}", ty.sol_type_name()))),
    })
}
fn from_sol(value: DynSolValue) -> Value {
    match value {
        DynSolValue::Bool(x) => Value::Bool(x),
        DynSolValue::Int(x, _) => Value::Word(B256::from(x.into_raw().to_be_bytes())),
        DynSolValue::Uint(x, _) => Value::Word(B256::from(x.to_be_bytes())),
        DynSolValue::Address(x) => Value::Word(x.into_word()),
        DynSolValue::FixedBytes(x, n) => Value::Bytes(x[..n].to_vec()),
        DynSolValue::Function(x) => Value::Bytes(x.as_slice().to_vec()),
        DynSolValue::Bytes(x) => Value::Bytes(x),
        DynSolValue::String(x) => Value::Bytes(x.into_bytes()),
        DynSolValue::Array(xs) | DynSolValue::FixedArray(xs) | DynSolValue::Tuple(xs) => {
            Value::Seq(xs.into_iter().map(from_sol).collect())
        }
    }
}
pub(crate) fn encode(abi: &str, signature: &str, args: &[Value]) -> Result<Vec<u8>> {
    let abi = parse(abi)?;
    let function = abi
        .functions()
        .find(|f| f.signature() == signature)
        .ok_or_else(|| err("unknown function signature"))?;
    if args.len() != function.inputs.len() {
        return Err(err("argument count mismatch"));
    }
    let values = function
        .inputs
        .iter()
        .zip(args)
        .map(|(p, v)| convert(&p.resolve().map_err(err)?, v, 0))
        .collect::<Result<Vec<_>>>()?;
    function.abi_encode_input(&values).map_err(err)
}
pub(crate) fn decode(abi: &str, signature: &str, bytes: &[u8]) -> Result<Value> {
    let abi = parse(abi)?;
    let f = abi
        .functions()
        .find(|f| f.signature() == signature)
        .ok_or_else(|| err("unknown function signature"))?;
    let values = f.abi_decode_output(bytes).map_err(err)?;
    // Reject invalid padding, integer widths, booleans, offsets and trailing data.
    if f.abi_encode_output(&values).map_err(err)? != bytes {
        return Err(err("noncanonical ABI return data"));
    }
    Ok(Value::Seq(values.into_iter().map(from_sol).collect()))
}
pub(crate) fn decode_error(abi: &str, signature: &str, bytes: &[u8]) -> Result<Value> {
    let abi = parse(abi)?;
    let e = abi
        .errors()
        .find(|e| e.signature() == signature)
        .ok_or_else(|| err("unknown error signature"))?;
    if bytes.get(..4) != Some(e.selector().as_slice()) {
        return Err(err("error selector mismatch"));
    }
    let values = e.abi_decode_input(&bytes[4..]).map_err(err)?;
    if e.abi_encode_input(&values).map_err(err)? != bytes {
        return Err(err("noncanonical ABI error data"));
    }
    Ok(Value::Seq(values.into_iter().map(from_sol).collect()))
}
pub(crate) fn decode_event(
    abi: &str,
    signature: &str,
    topics: &[B256],
    bytes: &[u8],
) -> Result<Value> {
    let abi = parse(abi)?;
    let event = abi
        .events()
        .find(|e| e.signature() == signature)
        .ok_or_else(|| err("unknown event signature"))?;
    let decoded = event
        .decode_log_parts(topics.iter().copied(), bytes)
        .map_err(err)?;
    let encoded = decoded.encode_log_data();
    if encoded.data.as_ref() != bytes || encoded.topics() != topics {
        return Err(err("noncanonical ABI event data"));
    }
    let mut indexed = decoded.indexed.into_iter();
    let mut body = decoded.body.into_iter();
    let mut values = Vec::new();
    for param in &event.inputs {
        let value = if param.indexed {
            indexed.next()
        } else {
            body.next()
        }
        .ok_or_else(|| err("missing event field"))?;
        values.push(from_sol(value));
    }
    Ok(Value::Seq(values))
}

pub(crate) fn encode_constructor(abi: &str, bytecode: &[u8], args: &[Value]) -> Result<Vec<u8>> {
    if bytecode.is_empty() {
        return Err(err("deployment bytecode is empty"));
    }
    let abi = parse(abi)?;
    let mut result = bytecode.to_vec();
    if let Some(constructor) = abi.constructor {
        if args.len() != constructor.inputs.len() {
            return Err(err("constructor argument count mismatch"));
        }
        let values = constructor
            .inputs
            .iter()
            .zip(args)
            .map(|(p, v)| convert(&p.resolve().map_err(err)?, v, 0))
            .collect::<Result<Vec<_>>>()?;
        result.extend(constructor.abi_encode_input(&values).map_err(err)?);
    } else if !args.is_empty() {
        return Err(err("contract has no constructor arguments"));
    }
    Ok(result)
}
