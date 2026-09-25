<?php
declare(strict_types=1);

$directory = $argv[1] ?? '';
$files = glob(rtrim($directory, '/') . '/*.php');
if ($files === false || count($files) !== 26) {
    throw new RuntimeException('expected 26 generated PHP metadata files');
}
foreach ($files as $file) require_once $file;
if (!defined('NativeBindings\\Token::ABI')) {
    throw new RuntimeException('Token ABI metadata missing');
}
if (class_exists('NativeBindings\\TokenClient') || method_exists('NativeBindings\\Token', 'encodeApprove')) {
    throw new RuntimeException('metadata mode emitted runtime wrappers');
}
echo "PHP metadata consumer passed\n";
