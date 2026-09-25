<?php
declare(strict_types=1);

$directory = $argv[1] ?? '';
$files = glob(rtrim($directory, '/') . '/*.php');
// Assert the tracked fixtures, rather than counting artifacts left by earlier runs.
$expected = [
    'BigInt', 'Boolean', 'Data', 'EdgeCases', 'Exchange', 'IEdgeCases',
    'IExchange', 'IRegistry', 'IToken', 'IVault', 'ListContract', 'Mod',
    'NamingCases', 'NativeCases', 'NativeNames', 'Override', 'Registry',
    'StringContract', 'Token', 'TupleAccount', 'TupleAmount', 'TupleCases',
    'Uint256', 'Vault',
];
$actual = $files === false ? [] : array_map(fn ($file) => basename($file, '.php'), $files);
$missing = array_diff($expected, $actual);
if ($missing !== []) {
    throw new RuntimeException('missing PHP metadata fixtures: ' . implode(', ', $missing));
}
foreach ($files as $file) require_once $file;
foreach ($expected as $name) {
    if (!defined('NativeBindings\\' . $name . '::ABI')) {
        throw new RuntimeException($name . ' ABI metadata missing');
    }
}
if (class_exists('NativeBindings\\TokenClient') || method_exists('NativeBindings\\Token', 'encodeApprove')) {
    throw new RuntimeException('metadata mode emitted runtime wrappers');
}
echo "PHP metadata consumer passed\n";
