//! Explicit ABI offset decoding for tuples that web3j reflection cannot traverse.

use super::super::Kotlin;
use abi_typegen_core::types::SolType;
use std::collections::BTreeSet;

pub(super) fn runtime() -> &'static str {
    r#"
    private sealed interface AbiLayout {
        val dynamic: Boolean
        val headBytes: Int
        fun read(hex: String, at: Int): Any
    }

    private class AbiAtom(
        override val dynamic: Boolean,
        private val decode: (String, Int) -> Any,
    ) : AbiLayout {
        override val headBytes: Int = 32
        override fun read(hex: String, at: Int): Any = decode(hex, at)
    }

    private class AbiArray(
        private val element: AbiLayout,
        private val fixedSize: Int?,
    ) : AbiLayout {
        override val dynamic: Boolean = fixedSize == null || element.dynamic
        override val headBytes: Int = if (dynamic) 32 else Math.multiplyExact(fixedSize!!, element.headBytes)
        override fun read(hex: String, at: Int): Any {
            val count = fixedSize ?: word(hex, at).intValueExact()
            require(count >= 0 && count <= hex.length / 64) { "array length exceeds ABI data" }
            val base = if (fixedSize == null) Math.addExact(at, 32) else at
            var cursor = base
            return List(count) {
                val value = if (element.dynamic) {
                    element.read(hex, Math.addExact(base, word(hex, cursor).intValueExact()))
                } else {
                    element.read(hex, cursor)
                }
                cursor = Math.addExact(cursor, element.headBytes)
                value
            }
        }
    }

    private class AbiTuple(
        private val fields: List<AbiLayout>,
        private val construct: (List<Any>) -> Any,
    ) : AbiLayout {
        override val dynamic: Boolean = fields.any { it.dynamic }
        override val headBytes: Int = if (dynamic) 32 else fields.fold(0) { size, field -> Math.addExact(size, field.headBytes) }
        override fun read(hex: String, at: Int): Any {
            var cursor = at
            val values = fields.map { field ->
                val value = if (field.dynamic) {
                    field.read(hex, Math.addExact(at, word(hex, cursor).intValueExact()))
                } else {
                    field.read(hex, cursor)
                }
                cursor = Math.addExact(cursor, field.headBytes)
                value
            }
            return construct(values)
        }
    }

    private fun word(hex: String, at: Int): java.math.BigInteger {
        val start = Math.multiplyExact(at, 2)
        require(start >= 0 && start <= hex.length - 64) { "truncated ABI word" }
        return java.math.BigInteger(hex.substring(start, start + 64), 16)
    }

    private fun decodeLayout(data: String, layout: AbiLayout): Any {
        require(data.startsWith("0x") && data.length % 2 == 0) { "invalid ABI hex" }
        return layout.read(data.substring(2), 0)
    }
"#
}

pub(super) fn expression(
    ty: &SolType,
    internal: Option<&str>,
    kotlin: &Kotlin<'_>,
    imports: &mut BTreeSet<&'static str>,
) -> String {
    match ty {
        SolType::Bool
        | SolType::StringType
        | SolType::Address
        | SolType::Uint(_)
        | SolType::Int(_)
        | SolType::Bytes
        | SolType::BytesN(_) => {
            let class = kotlin.web3j_class(ty, internal, imports);
            let value = match ty {
                SolType::Bytes | SolType::BytesN(_) => "decoded".to_string(),
                SolType::Bool
                | SolType::StringType
                | SolType::Address
                | SolType::Uint(_)
                | SolType::Int(_) => "decoded.value".to_string(),
                SolType::Array(_) | SolType::FixedArray(_, _) | SolType::Tuple(_) => {
                    unreachable!("matched atomic ABI type")
                }
            };
            format!(
                "AbiAtom({}) {{ hex, at -> val decoded = org.web3j.abi.TypeDecoder.decode(hex, Math.multiplyExact(at, 2), {class}::class.java); {value} }}",
                super::super::is_dynamic(ty)
            )
        }
        SolType::Array(inner) => format!(
            "AbiArray({}, null)",
            expression(inner, internal, kotlin, imports)
        ),
        SolType::FixedArray(inner, size) => format!(
            "AbiArray({}, {size})",
            expression(inner, internal, kotlin, imports)
        ),
        SolType::Tuple(components) => {
            let name = &kotlin.tuple_names[kotlin.registry.name(components, internal)];
            format!("layout{name}()")
        }
    }
}
