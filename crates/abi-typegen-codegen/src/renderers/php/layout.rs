//! PHP RPC wrappers plus a generated Solidity ABI codec.

use super::{Php, indexed_hash, php_ident, php_string};
use crate::naming::{Scope, param_names};
use abi_typegen_core::types::{ContractIr, StateMutability};

pub(super) fn render_support(php: &Php<'_>) -> String {
    let options = php.transaction_options;
    let client = php.client;
    format!(
        r#"
/** Gas settings and payable amount for a write transaction. */
final readonly class {options}
{{
    public \Brick\Math\BigInteger $value;

    public function __construct(
        public \Brick\Math\BigInteger $gasPrice,
        public \Brick\Math\BigInteger $gasLimit,
        ?\Brick\Math\BigInteger $value = null,
    ) {{
        $this->value = $value ?? \Brick\Math\BigInteger::of(0);
        if ($this->gasPrice->isNegative() || $this->gasLimit->isNegative() || $this->value->isNegative()) {{
            throw new \InvalidArgumentException('transaction quantities must be nonnegative');
        }}
    }}
}}

/** JSON-RPC handle for a deployed contract, with optional local transaction signing. */
final class {client}
{{
    public function __construct(
        public readonly string $address,
        public readonly string $rpcUrl,
        public readonly ?string $privateKey = null,
        public readonly ?string $fromAddress = null,
        public readonly ?int $chainId = null,
    ) {{}}

    public function call(string $data, string $block = 'latest'): string
    {{
        $request = ['to' => $this->address, 'data' => $data];
        if ($this->fromAddress !== null) $request['from'] = $this->fromAddress;
        $result = $this->rpc('eth_call', [$request, $block]);
        if (!is_string($result)) throw new \RuntimeException('eth_call returned a non-string result');
        return $result;
    }}

    public function send(string $data, {options} $options): string
    {{
        if ($this->privateKey === null || $this->fromAddress === null || $this->chainId === null) {{
            throw new \LogicException('write transactions require privateKey, fromAddress, and chainId');
        }}
        $nonce = $this->rpc('eth_getTransactionCount', [$this->fromAddress, 'pending']);
        if (!is_string($nonce)) throw new \RuntimeException('eth_getTransactionCount returned a non-string result');
        $transaction = new \Web3p\EthereumTx\Transaction([
            'nonce' => $nonce,
            'from' => $this->fromAddress,
            'to' => $this->address,
            'gas' => self::quantity($options->gasLimit),
            'gasPrice' => self::quantity($options->gasPrice),
            'value' => self::quantity($options->value),
            'data' => $data,
            'chainId' => $this->chainId,
        ]);
        $key = str_starts_with($this->privateKey, '0x') ? substr($this->privateKey, 2) : $this->privateKey;
        $raw = '0x' . $transaction->sign($key);
        $hash = $this->rpc('eth_sendRawTransaction', [$raw]);
        if (!is_string($hash)) throw new \RuntimeException('eth_sendRawTransaction returned a non-string result');
        return $hash;
    }}

    public function gasPrice(): \Brick\Math\BigInteger
    {{
        $value = $this->rpc('eth_gasPrice');
        if (!is_string($value)) throw new \RuntimeException('eth_gasPrice returned a non-string result');
        return self::quantityToBigInteger($value);
    }}

    public function getTransactionReceipt(string $hash): ?array
    {{
        $value = $this->rpc('eth_getTransactionReceipt', [$hash]);
        if ($value === null) return null;
        if (!is_array($value)) throw new \RuntimeException('eth_getTransactionReceipt returned a non-object result');
        return $value;
    }}

    /** @return list<array<string,mixed>> */
    public function getLogs(array $filter): array
    {{
        $value = $this->rpc('eth_getLogs', [$filter]);
        if (!is_array($value)) throw new \RuntimeException('eth_getLogs returned a non-array result');
        return $value;
    }}

    public function rpc(string $method, array $params = []): mixed
    {{
        $body = json_encode(['jsonrpc' => '2.0', 'id' => 1, 'method' => $method, 'params' => $params], JSON_THROW_ON_ERROR);
        $curl = curl_init($this->rpcUrl);
        if ($curl === false) throw new \RuntimeException('failed to initialize cURL');
        curl_setopt_array($curl, [
            CURLOPT_POST => true,
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_HTTPHEADER => ['Content-Type: application/json'],
            CURLOPT_POSTFIELDS => $body,
            CURLOPT_CONNECTTIMEOUT => 5,
            CURLOPT_TIMEOUT => 30,
        ]);
        $response = curl_exec($curl);
        if ($response === false) {{
            $message = curl_error($curl);
            throw new \RuntimeException('JSON-RPC transport error: ' . $message);
        }}
        $status = curl_getinfo($curl, CURLINFO_RESPONSE_CODE);
        if ($status < 200 || $status >= 300) throw new \RuntimeException('JSON-RPC HTTP status ' . $status);
        $decoded = json_decode($response, true, 512, JSON_THROW_ON_ERROR);
        if (isset($decoded['error'])) {{
            $message = is_array($decoded['error']) ? ($decoded['error']['message'] ?? json_encode($decoded['error'])) : (string)$decoded['error'];
            throw new \RuntimeException('JSON-RPC error: ' . $message);
        }}
        return $decoded['result'] ?? null;
    }}

    private static function quantity(\Brick\Math\BigInteger $value): string
    {{
        if ($value->isNegative()) throw new \InvalidArgumentException('quantity must be nonnegative');
        $hex = ltrim(strtolower($value->toBase(16)), '0');
        return '0x' . ($hex === '' ? '0' : $hex);
    }}

    private static function quantityToBigInteger(string $value): \Brick\Math\BigInteger
    {{
        if (!preg_match('/^0x[0-9a-f]+$/i', $value)) throw new \InvalidArgumentException('invalid JSON-RPC quantity');
        $hex = substr($value, 2);
        return $hex === '' ? \Brick\Math\BigInteger::of(0) : \Brick\Math\BigInteger::fromBase($hex, 16);
    }}
}}
"#
    )
}

pub(super) fn render_methods(ir: &ContractIr, php: &Php<'_>) -> String {
    let mut out = String::from(CODEC);

    for (tuple_index, def) in php.registry.defs().iter().enumerate() {
        let name = &php.tuple_names[&def.name];
        let stem = &php.tuple_stems[tuple_index];
        let layout_fields = def
            .components
            .iter()
            .map(|component| php.layout_expr(&component.ty, component.internal_type.as_deref()))
            .collect::<Vec<_>>()
            .join(", ");
        let layout = format!("['kind' => 'tuple', 'fields' => [{layout_fields}]]");
        let params = def
            .components
            .iter()
            .map(|component| abi_typegen_core::types::AbiParam {
                name: component.name.clone(),
                ty: component.ty.clone(),
                internal_type: component.internal_type.clone(),
            })
            .collect::<Vec<_>>();
        let fields = php.fields(&params);
        let encoded = def
            .components
            .iter()
            .zip(&fields)
            .map(|(component, field)| {
                php.to_abi(
                    &format!("$value->{}", field.name),
                    &component.ty,
                    component.internal_type.as_deref(),
                    0,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let encoded = format!("[{encoded}]");
        let decoded_args = def
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| {
                php.decode_abi_expr(
                    &format!("$values[0][{index}]"),
                    &component.ty,
                    component.internal_type.as_deref(),
                    0,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let decoded = format!("new {name}({decoded_args})");
        out.push_str(&format!(
            "\n    /** Encodes one `{name}` ABI value. */\n    public static function encode{stem}Value({name} $value): string\n    {{\n        return '0x' . self::encodeSequence([{layout}], [{encoded}]);\n    }}\n"
        ));
        out.push_str(&format!(
            "    /** Decodes one `{name}` ABI value. */\n    public static function decode{stem}Value(string $data): {name}\n    {{\n        $values = self::decodeTop($data, [{layout}]);\n        return {decoded};\n    }}\n"
        ));
    }

    for (index, function) in ir.functions.iter().enumerate() {
        let stem = &php.function_stems[index];
        let fields = php.fields_with_reserved(&function.inputs, &["client", "options"]);
        let args = fields
            .iter()
            .map(|field| format!("{} ${}", field.ty, field.name))
            .collect::<Vec<_>>()
            .join(", ");
        let names = fields
            .iter()
            .map(|field| format!("${}", field.name))
            .collect::<Vec<_>>();
        let layouts = function
            .inputs
            .iter()
            .map(|param| php.layout_expr(&param.ty, param.internal_type.as_deref()))
            .collect::<Vec<_>>()
            .join(", ");
        let values = function
            .inputs
            .iter()
            .zip(&fields)
            .map(|(param, field)| {
                php.to_abi(
                    &format!("${}", field.name),
                    &param.ty,
                    param.internal_type.as_deref(),
                    0,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let selector = function.selector().to_string();
        out.push_str(&format!(
            "\n    /** Encodes `{}` calldata without RPC. */\n    public static function encode{stem}({args}): string\n    {{\n        return {} . self::encodeSequence([{layouts}], [{values}]);\n    }}\n",
            function.signature(),
            php_string(&selector)
        ));

        let result_ty = match function.outputs.len() {
            0 => "void".to_string(),
            1 => php.field_type(
                &function.outputs[0].ty,
                function.outputs[0].internal_type.as_deref(),
            ),
            _ => php.result_names[index]
                .as_ref()
                .expect("multi-result class")
                .clone(),
        };
        out.push_str(&format!(
            "    /** Decodes `{}` ABI output. */\n    public static function decode{stem}Result(string $data): {result_ty}\n    {{\n",
            function.signature()
        ));
        if function.outputs.is_empty() {
            out.push_str(
                "        if ($data !== '' && strtolower($data) !== '0x') throw new \\InvalidArgumentException('unexpected void result');\n        return;\n",
            );
        } else {
            let output_layouts = function
                .outputs
                .iter()
                .map(|param| php.layout_expr(&param.ty, param.internal_type.as_deref()))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "        $values = self::decodeTop($data, [{output_layouts}]);\n        if (count($values) !== {}) throw new \\InvalidArgumentException('incorrect result arity');\n",
                function.outputs.len()
            ));
            if function.outputs.len() == 1 {
                let value = php.decode_abi_expr(
                    "$values[0]",
                    &function.outputs[0].ty,
                    function.outputs[0].internal_type.as_deref(),
                    0,
                );
                out.push_str(&format!("        return {value};\n"));
            } else {
                let result_name = php.result_names[index].as_ref().expect("result name");
                let values = function
                    .outputs
                    .iter()
                    .enumerate()
                    .map(|(output_index, param)| {
                        php.decode_abi_expr(
                            &format!("$values[{output_index}]"),
                            &param.ty,
                            param.internal_type.as_deref(),
                            0,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("        return new {result_name}({values});\n"));
            }
        }
        out.push_str("    }\n");

        match function.state_mutability {
            StateMutability::View | StateMutability::Pure => {
                let full_args = if args.is_empty() {
                    format!("{} $client", php.client)
                } else {
                    format!("{} $client, {args}", php.client)
                };
                let invocation = names.join(", ");
                let encode_call = format!("self::encode{stem}({invocation})");
                let return_stmt = if result_ty == "void" {
                    format!("self::decode{stem}Result($client->call({encode_call}));")
                } else {
                    format!("return self::decode{stem}Result($client->call({encode_call}));")
                };
                out.push_str(&format!(
                    "    /** Performs a typed read call. */\n    public static function call{stem}({full_args}): {result_ty}\n    {{\n        {return_stmt}\n    }}\n"
                ));
            }
            StateMutability::NonPayable | StateMutability::Payable => {
                let full_args = if args.is_empty() {
                    format!(
                        "{} $client, {} $options",
                        php.client, php.transaction_options
                    )
                } else {
                    format!(
                        "{} $client, {args}, {} $options",
                        php.client, php.transaction_options
                    )
                };
                let invocation = names.join(", ");
                let check = if function.state_mutability == StateMutability::NonPayable {
                    "        if (!$options->value->isZero()) throw new \\InvalidArgumentException('nonpayable function cannot receive value');\n"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "    /** Signs and submits a typed write transaction. */\n    public static function send{stem}({full_args}): string\n    {{\n{check}        return $client->send(self::encode{stem}({invocation}), $options);\n    }}\n"
                ));
            }
        }
    }

    for (event_index, event) in ir.events.iter().enumerate() {
        let stem = &php.event_stems[event_index];
        let normal_name = &php.event_names[event_index];
        let result_name = php.decoded_event_names[event_index]
            .as_ref()
            .unwrap_or(normal_name);
        let topic_start = if event.anonymous { 0 } else { 1 };
        let indexed_count = event.inputs.iter().filter(|param| param.indexed).count();
        let nonindexed = event
            .inputs
            .iter()
            .filter(|param| !param.indexed)
            .collect::<Vec<_>>();
        let layouts = nonindexed
            .iter()
            .map(|param| php.layout_expr(&param.ty, param.internal_type.as_deref()))
            .collect::<Vec<_>>()
            .join(", ");

        out.push_str(&format!(
            "\n    /** Decodes one `{}` event log, or null for another topic. */\n    public static function decode{stem}Event(array|object $log): ?{result_name}\n    {{\n        $topics = self::logField($log, 'topics');\n        $data = self::logField($log, 'data');\n        if (!is_array($topics) || !is_string($data)) throw new \\InvalidArgumentException('invalid log object');\n",
            event.signature()
        ));
        if !event.anonymous {
            out.push_str(&format!(
                "        if ($topics === [] || !is_string($topics[0]) || strcasecmp($topics[0], {}) !== 0) return null;\n",
                php_string(&event.topic0().to_string())
            ));
        }
        out.push_str(&format!(
            "        if (count($topics) !== {}) return null;\n        $values = self::decodeTop($data, [{layouts}]);\n",
            topic_start + indexed_count
        ));

        let mut topic_i = topic_start;
        let mut data_i = 0usize;
        let mut args_out = Vec::new();
        for param in &event.inputs {
            if param.indexed {
                if indexed_hash(&param.ty) {
                    args_out.push(format!("$topics[{topic_i}]"));
                } else {
                    let layout = php.layout_expr(&param.ty, param.internal_type.as_deref());
                    let decoded = format!("self::decodeTop($topics[{topic_i}], [{layout}])[0]");
                    args_out.push(php.decode_abi_expr(
                        &decoded,
                        &param.ty,
                        param.internal_type.as_deref(),
                        0,
                    ));
                }
                topic_i += 1;
            } else {
                args_out.push(php.decode_abi_expr(
                    &format!("$values[{data_i}]"),
                    &param.ty,
                    param.internal_type.as_deref(),
                    0,
                ));
                data_i += 1;
            }
        }
        out.push_str(&format!(
            "        return new {result_name}({});\n    }}\n",
            args_out.join(", ")
        ));

        let indexed = event
            .inputs
            .iter()
            .filter(|param| param.indexed)
            .collect::<Vec<_>>();
        let indexed_names = param_names(indexed.iter().map(|param| {
            (
                param.name.as_str(),
                &param.ty,
                php.registry
                    .name_of_type(&param.ty, param.internal_type.as_deref()),
            )
        }));
        let mut scope = Scope::with_reserved(["address", "fromBlock", "toBlock"]);
        let mut filter_args = vec![
            "string $address".to_string(),
            "string $fromBlock = 'earliest'".to_string(),
            "string $toBlock = 'latest'".to_string(),
        ];
        let mut topic_lines = String::new();
        if !event.anonymous {
            topic_lines.push_str(&format!(
                "        $topics[] = {};\n",
                php_string(&event.topic0().to_string())
            ));
        }
        for (param, name) in indexed.iter().zip(indexed_names) {
            let name = scope.claim(&php_ident(&name));
            let ty = if indexed_hash(&param.ty) {
                "string".to_string()
            } else {
                php.field_type(&param.ty, param.internal_type.as_deref())
            };
            filter_args.push(format!("?{ty} ${name} = null"));
            if indexed_hash(&param.ty) {
                topic_lines.push_str(&format!("        $topics[] = ${name};\n"));
            } else {
                let layout = php.layout_expr(&param.ty, param.internal_type.as_deref());
                let encoded = php.to_abi(
                    &format!("${name}"),
                    &param.ty,
                    param.internal_type.as_deref(),
                    0,
                );
                topic_lines.push_str(&format!(
                    "        $topics[] = ${name} === null ? null : '0x' . self::encodeSequence([{layout}], [{encoded}]);\n"
                ));
            }
        }
        out.push_str(&format!(
            "    /** Builds a positional JSON-RPC event filter. */\n    public static function filter{stem}Event({}): array\n    {{\n        $topics = [];\n{topic_lines}        return ['address' => $address, 'fromBlock' => $fromBlock, 'toBlock' => $toBlock, 'topics' => $topics];\n    }}\n",
            filter_args.join(", ")
        ));
    }

    for (error_index, error) in ir.errors.iter().enumerate() {
        let stem = &php.error_stems[error_index];
        let class_name = &php.error_names[error_index];
        let layouts = error
            .inputs
            .iter()
            .map(|param| php.layout_expr(&param.ty, param.internal_type.as_deref()))
            .collect::<Vec<_>>()
            .join(", ");
        let args = error
            .inputs
            .iter()
            .enumerate()
            .map(|(index, param)| {
                php.decode_abi_expr(
                    &format!("$values[{index}]"),
                    &param.ty,
                    param.internal_type.as_deref(),
                    0,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "\n    /** Decodes `{}` custom error, or null for another selector. */\n    public static function decode{stem}Error(string $data): ?{class_name}\n    {{\n        if (strlen($data) < 10 || strcasecmp(substr($data, 0, 10), {}) !== 0) return null;\n        $values = self::decodeTop('0x' . substr($data, 10), [{layouts}]);\n        return new {class_name}({args});\n    }}\n",
            error.signature(),
            php_string(&error.selector().to_string())
        ));
    }

    out
}

const CODEC: &str = r#"

    /** @return list<mixed> */
    private static function decodeTop(string $data, array $layouts): array
    {
        if (!str_starts_with($data, '0x') || (strlen($data) % 2) !== 0) {
            throw new \InvalidArgumentException('invalid ABI hex');
        }
        $hex = substr($data, 2);
        if ($hex !== '' && !ctype_xdigit($hex)) throw new \InvalidArgumentException('invalid ABI hex');
        $hex = strtolower($hex);
        $values = self::decodeSequenceAt($hex, 0, $layouts);
        if (self::encodeSequence($layouts, $values) !== $hex) {
            throw new \InvalidArgumentException('noncanonical or trailing ABI data');
        }
        return $values;
    }

    /** @param list<array<string,mixed>> $layouts @param list<mixed> $values */
    private static function encodeSequence(array $layouts, array $values): string
    {
        if (count($layouts) !== count($values)) throw new \InvalidArgumentException('incorrect ABI value arity');
        $headBytes = 0;
        foreach ($layouts as $layout) $headBytes = self::checkedAdd($headBytes, self::layoutDynamic($layout) ? 32 : self::staticSize($layout));
        $head = '';
        $tail = '';
        $cursor = $headBytes;
        foreach ($layouts as $index => $layout) {
            if (self::layoutDynamic($layout)) {
                $encoded = self::encodeValue($layout, $values[$index]);
                $head .= self::uintWord(\Brick\Math\BigInteger::of($cursor), 256);
                $tail .= $encoded;
                $cursor = self::checkedAdd($cursor, intdiv(strlen($encoded), 2));
            } else {
                $head .= self::encodeValue($layout, $values[$index]);
            }
        }
        return $head . $tail;
    }

    private static function encodeValue(array $layout, mixed $value): string
    {
        if ($layout['kind'] === 'atom') {
            $type = $layout['type'];
            if ($type === 'bool') {
                if (!is_bool($value)) throw new \InvalidArgumentException('bool ABI value must be a bool');
                return str_repeat('0', 63) . ($value ? '1' : '0');
            }
            if ($type === 'address') {
                if (!is_string($value) || !preg_match('/^0x[0-9a-f]{40}$/i', $value)) throw new \InvalidArgumentException('invalid address');
                return str_repeat('0', 24) . strtolower(substr($value, 2));
            }
            if ($type === 'string') {
                if (!is_string($value)) throw new \InvalidArgumentException('string ABI value must be a string');
                $hex = bin2hex($value);
                return self::lengthPrefixedBytes($hex);
            }
            if ($type === 'bytes') return self::lengthPrefixedBytes(self::bytesHex($value));
            if (preg_match('/^bytes([0-9]+)$/', $type, $match)) {
                $size = (int)$match[1];
                if ($size < 1 || $size > 32) throw new \InvalidArgumentException('invalid fixed bytes width');
                $hex = self::bytesHex($value);
                if (strlen($hex) !== $size * 2) throw new \InvalidArgumentException($type . ' requires exactly ' . $size . ' bytes');
                return str_pad($hex, 64, '0', STR_PAD_RIGHT);
            }
            if (preg_match('/^uint([0-9]+)$/', $type, $match)) return self::uintWord(self::bigInteger($value), (int)$match[1]);
            if (preg_match('/^int([0-9]+)$/', $type, $match)) return self::intWord(self::bigInteger($value), (int)$match[1]);
            throw new \InvalidArgumentException('unknown ABI atom ' . $type);
        }
        if ($layout['kind'] === 'tuple') {
            if (!is_array($value)) throw new \InvalidArgumentException('tuple ABI value must be an array');
            return self::encodeSequence($layout['fields'], array_values($value));
        }
        if ($layout['kind'] === 'array') {
            if (!is_array($value)) throw new \InvalidArgumentException('array ABI value must be an array');
            $values = array_values($value);
            $size = $layout['size'];
            if ($size !== null && count($values) !== $size) throw new \InvalidArgumentException('incorrect fixed array length');
            $layouts = array_fill(0, count($values), $layout['element']);
            $body = self::encodeSequence($layouts, $values);
            return $size === null ? self::uintWord(\Brick\Math\BigInteger::of(count($values)), 256) . $body : $body;
        }
        throw new \InvalidArgumentException('unknown ABI layout');
    }

    /** @return list<mixed> */
    private static function decodeSequenceAt(string $hex, int $base, array $layouts): array
    {
        $cursor = $base;
        $values = [];
        foreach ($layouts as $layout) {
            if (self::layoutDynamic($layout)) {
                $offset = self::smallWord($hex, $cursor);
                $values[] = self::decodeValue($hex, self::checkedAdd($base, $offset), $layout);
                $cursor = self::checkedAdd($cursor, 32);
            } else {
                $values[] = self::decodeValue($hex, $cursor, $layout);
                $cursor = self::checkedAdd($cursor, self::staticSize($layout));
            }
        }
        return $values;
    }

    private static function decodeValue(string $hex, int $at, array $layout): mixed
    {
        if ($layout['kind'] === 'atom') {
            $type = $layout['type'];
            if ($type === 'string' || $type === 'bytes') {
                $length = self::smallWord($hex, $at);
                $start = self::checkedAdd($at, 32);
                self::ensureRange($hex, $start, $length);
                $payload = substr($hex, $start * 2, $length * 2);
                if ($type === 'bytes') return '0x' . $payload;
                $decoded = hex2bin($payload);
                if ($decoded === false) throw new \InvalidArgumentException('invalid ABI string bytes');
                return $decoded;
            }
            $word = self::wordHex($hex, $at);
            if ($type === 'bool') {
                if ($word === str_repeat('0', 64)) return false;
                if ($word === str_repeat('0', 63) . '1') return true;
                throw new \InvalidArgumentException('invalid ABI bool');
            }
            if ($type === 'address') {
                if (substr($word, 0, 24) !== str_repeat('0', 24)) throw new \InvalidArgumentException('invalid ABI address padding');
                return '0x' . substr($word, 24);
            }
            if (preg_match('/^bytes([0-9]+)$/', $type, $match)) {
                $size = (int)$match[1];
                if ($size < 1 || $size > 32) throw new \InvalidArgumentException('invalid fixed bytes width');
                if (substr($word, $size * 2) !== str_repeat('0', 64 - $size * 2)) throw new \InvalidArgumentException('invalid fixed bytes padding');
                return '0x' . substr($word, 0, $size * 2);
            }
            if (preg_match('/^uint([0-9]+)$/', $type, $match)) {
                $value = \Brick\Math\BigInteger::fromBase($word, 16);
                $bits = (int)$match[1];
                if ($value->isGreaterThanOrEqualTo(self::pow2($bits))) throw new \InvalidArgumentException('decoded uint exceeds declared width');
                return $value;
            }
            if (preg_match('/^int([0-9]+)$/', $type, $match)) {
                $unsigned = \Brick\Math\BigInteger::fromBase($word, 16);
                $value = hexdec($word[0]) >= 8 ? $unsigned->minus(self::pow2(256)) : $unsigned;
                $bits = (int)$match[1];
                $limit = self::pow2($bits - 1);
                if ($value->isLessThan($limit->negated()) || $value->isGreaterThanOrEqualTo($limit)) throw new \InvalidArgumentException('decoded int exceeds declared width');
                return $value;
            }
            throw new \InvalidArgumentException('unknown ABI atom ' . $type);
        }
        if ($layout['kind'] === 'tuple') return self::decodeSequenceAt($hex, $at, $layout['fields']);
        if ($layout['kind'] === 'array') {
            $size = $layout['size'];
            if ($size === null) {
                $count = self::smallWord($hex, $at);
                $base = self::checkedAdd($at, 32);
            } else {
                $count = $size;
                $base = $at;
            }
            if ($count < 0 || $count > intdiv(strlen($hex), 64) + 1) throw new \InvalidArgumentException('array length exceeds ABI data');
            $element = $layout['element'];
            $values = [];
            $cursor = $base;
            for ($index = 0; $index < $count; $index++) {
                if (self::layoutDynamic($element)) {
                    $offset = self::smallWord($hex, $cursor);
                    $values[] = self::decodeValue($hex, self::checkedAdd($base, $offset), $element);
                    $cursor = self::checkedAdd($cursor, 32);
                } else {
                    $values[] = self::decodeValue($hex, $cursor, $element);
                    $cursor = self::checkedAdd($cursor, self::staticSize($element));
                }
            }
            return $values;
        }
        throw new \InvalidArgumentException('unknown ABI layout');
    }

    private static function layoutDynamic(array $layout): bool
    {
        if ($layout['kind'] === 'atom') return $layout['type'] === 'string' || $layout['type'] === 'bytes';
        if ($layout['kind'] === 'tuple') {
            foreach ($layout['fields'] as $field) if (self::layoutDynamic($field)) return true;
            return false;
        }
        if ($layout['kind'] === 'array') return $layout['size'] === null || self::layoutDynamic($layout['element']);
        throw new \InvalidArgumentException('unknown ABI layout');
    }

    private static function staticSize(array $layout): int
    {
        if (self::layoutDynamic($layout)) throw new \InvalidArgumentException('dynamic ABI value has no static size');
        if ($layout['kind'] === 'atom') return 32;
        if ($layout['kind'] === 'tuple') {
            $size = 0;
            foreach ($layout['fields'] as $field) $size = self::checkedAdd($size, self::staticSize($field));
            return $size;
        }
        if ($layout['kind'] === 'array') return self::checkedMul($layout['size'], self::staticSize($layout['element']));
        throw new \InvalidArgumentException('unknown ABI layout');
    }

    private static function uintWord(\Brick\Math\BigInteger $value, int $bits): string
    {
        self::validateIntegerBits($bits);
        if ($value->isNegative() || $value->isGreaterThanOrEqualTo(self::pow2($bits))) throw new \InvalidArgumentException('uint value is out of range');
        return str_pad(strtolower($value->toBase(16)), 64, '0', STR_PAD_LEFT);
    }

    private static function intWord(\Brick\Math\BigInteger $value, int $bits): string
    {
        self::validateIntegerBits($bits);
        $limit = self::pow2($bits - 1);
        if ($value->isLessThan($limit->negated()) || $value->isGreaterThanOrEqualTo($limit)) throw new \InvalidArgumentException('int value is out of range');
        $encoded = $value->isNegative() ? $value->plus(self::pow2(256)) : $value;
        return str_pad(strtolower($encoded->toBase(16)), 64, '0', STR_PAD_LEFT);
    }

    private static function validateIntegerBits(int $bits): void
    {
        if ($bits < 8 || $bits > 256 || ($bits % 8) !== 0) throw new \InvalidArgumentException('invalid Solidity integer width');
    }

    private static function pow2(int $bits): \Brick\Math\BigInteger
    {
        return \Brick\Math\BigInteger::of(2)->power($bits);
    }

    private static function bigInteger(mixed $value): \Brick\Math\BigInteger
    {
        if ($value instanceof \Brick\Math\BigInteger) return $value;
        if (is_int($value)) return \Brick\Math\BigInteger::of($value);
        if (is_string($value)) {
            if (preg_match('/^-?0x[0-9a-f]+$/i', $value)) {
                $negative = str_starts_with($value, '-');
                $hex = substr($value, $negative ? 3 : 2);
                $result = \Brick\Math\BigInteger::fromBase($hex, 16);
                return $negative ? $result->negated() : $result;
            }
            if (preg_match('/^-?[0-9]+$/', $value)) return \Brick\Math\BigInteger::of($value);
        }
        throw new \InvalidArgumentException('integer ABI value must be BigInteger, int, or decimal/hex string');
    }

    private static function lengthPrefixedBytes(string $hex): string
    {
        $length = intdiv(strlen($hex), 2);
        $padded = str_pad($hex, intdiv($length + 31, 32) * 64, '0', STR_PAD_RIGHT);
        return self::uintWord(\Brick\Math\BigInteger::of($length), 256) . $padded;
    }

    private static function bytesHex(mixed $value): string
    {
        if (!is_string($value)) throw new \InvalidArgumentException('bytes ABI value must be a string');
        if (str_starts_with($value, '0x')) {
            $hex = substr($value, 2);
            if ((strlen($hex) % 2) !== 0 || ($hex !== '' && !ctype_xdigit($hex))) throw new \InvalidArgumentException('invalid bytes hex');
            return strtolower($hex);
        }
        return bin2hex($value);
    }

    private static function wordHex(string $hex, int $at): string
    {
        self::ensureRange($hex, $at, 32);
        return substr($hex, $at * 2, 64);
    }

    private static function smallWord(string $hex, int $at): int
    {
        $value = \Brick\Math\BigInteger::fromBase(self::wordHex($hex, $at), 16);
        if ($value->isGreaterThan(\Brick\Math\BigInteger::of(PHP_INT_MAX))) throw new \InvalidArgumentException('ABI offset exceeds PHP integer range');
        return $value->toInt();
    }

    private static function ensureRange(string $hex, int $at, int $length): void
    {
        if ($at < 0 || $length < 0) throw new \InvalidArgumentException('negative ABI range');
        $end = self::checkedAdd($at, $length);
        if ($end > intdiv(strlen($hex), 2)) throw new \InvalidArgumentException('truncated ABI data');
    }

    private static function checkedAdd(int $left, int $right): int
    {
        if ($right > 0 && $left > PHP_INT_MAX - $right) throw new \InvalidArgumentException('ABI offset overflow');
        return $left + $right;
    }

    private static function checkedMul(int $left, int $right): int
    {
        if ($left < 0 || $right < 0) throw new \InvalidArgumentException('negative ABI size');
        if ($left !== 0 && $right > intdiv(PHP_INT_MAX, $left)) throw new \InvalidArgumentException('ABI size overflow');
        return $left * $right;
    }

    private static function logField(array|object $log, string $name): mixed
    {
        if (is_array($log)) return $log[$name] ?? null;
        return $log->{$name} ?? null;
    }
"#;
