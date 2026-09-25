<?php
declare(strict_types=1);

require __DIR__ . '/vendor/autoload.php';
foreach (glob(__DIR__ . '/Generated/*.php') ?: [] as $file) require_once $file;

use Brick\Math\BigInteger;
use NativeBindings\EdgeCases;
use NativeBindings\EdgeCasesComplexStruct;
use NativeBindings\NativeCases;
use NativeBindings\NativeCasesGrid;
use NativeBindings\NamingCases;
use NativeBindings\Token;
use NativeBindings\TokenClient;
use NativeBindings\TokenTransactionOptions;

function check(bool $condition, string $message): void
{
    if (!$condition) throw new RuntimeException($message);
}

function rejects(callable $body, string $message): void
{
    try {
        $body();
    } catch (InvalidArgumentException $error) {
        return;
    }
    throw new RuntimeException($message);
}

function word(string $hex): string
{
    return str_pad($hex, 64, '0', STR_PAD_LEFT);
}

function offline(): void
{
    $spender = '0x0000000000000000000000000000000000000002';
    $amount = BigInteger::of('340282366920938463463374607431768211456');
    $calldata = Token::encodeApprove($spender, $amount);
    check($calldata === '0x095ea7b3' . word('2') . word('100000000000000000000000000000000'), 'big integer calldata');
    check(Token::decodeBalanceOfResult('0x' . word('100000000000000000000000000000000'))->isEqualTo($amount), 'big integer result');
    rejects(static fn() => Token::decodeBalanceOfResult('0x' . word('2') . word('0')), 'trailing result accepted');
    rejects(static fn() => Token::decodeMintResult('0x' . word('0')), 'nonempty void result accepted');
    check(Token::decodeInvalidRecipientError(Token::INVALID_RECIPIENT_ERROR_SELECTOR) instanceof \NativeBindings\TokenInvalidRecipientError, 'zero-arg error');
    rejects(static fn() => Token::decodeInvalidRecipientError(Token::INVALID_RECIPIENT_ERROR_SELECTOR . word('0')), 'trailing error data accepted');

    $input = new EdgeCasesComplexStruct(BigInteger::of(7), $spender, '0x' . str_repeat('11', 32), true, 'hello', BigInteger::of(9));
    $tuple = EdgeCases::encodeComplexStructValue($input);
    $decoded = EdgeCases::decodeComplexStructValue($tuple);
    check($decoded->id->isEqualTo(BigInteger::of(7)) && $decoded->name === 'hello' && $decoded->active, 'tuple roundtrip');
    check(str_starts_with(EdgeCases::encodeProcessComplex($input), EdgeCases::PROCESS_COMPLEX_SELECTOR), 'tuple calldata');
    $nested = EdgeCases::decodeNestedArrayResult('0x' . word('20') . word('1') . word('20') . word('2') . word('1') . word('2'));
    check(count($nested) === 1 && $nested[0][1]->isEqualTo(BigInteger::of(2)), 'nested array result');
    $error = EdgeCases::decodeUnauthorizedError(EdgeCases::UNAUTHORIZED_ERROR_SELECTOR);
    check($error instanceof \NativeBindings\EdgeCasesUnauthorizedError, 'custom error');
    check(EdgeCases::decodeInvalidInputError(EdgeCases::UNAUTHORIZED_ERROR_SELECTOR) === null, 'error selector mismatch');
    $payload = word('40') . word('80') . word('3') . str_pad(bin2hex('bad'), 64, '0') . word('1') . str_pad('01', 64, '0');
    $typedError = EdgeCases::decodeInvalidInputError(EdgeCases::INVALID_INPUT_ERROR_SELECTOR . $payload);
    check($typedError !== null && $typedError->reason === 'bad' && $typedError->data === '0x01', 'payload custom error');

    $grid = new NativeCasesGrid([[BigInteger::of(1), BigInteger::of(2)]], [true, false], ['0x' . str_repeat('ab', 32)]);
    $decodedGrid = NativeCases::decodeGridValue(NativeCases::encodeGridValue($grid));
    check($decodedGrid->rows[0][1]->isEqualTo(BigInteger::of(2)) && $decodedGrid->flags === [true, false], 'nested tuple arrays');
    check(substr(NamingCases::encodeFooBar(true), 0, 10) !== substr(NamingCases::encodeFooBar2(BigInteger::of(1)), 0, 10), 'overload selectors');

    $topic = '0x' . str_repeat('ab', 32);
    $filter = NativeCases::filterIndexedReferencesEvent($spender, '0x1', '0x2', $topic);
    check($filter['topics'][1] === $topic && $filter['topics'][2] === null, 'indexed reference hash filter');
    $decodedLog = NativeCases::decodeIndexedReferencesEvent([
        'topics' => [NativeCases::INDEXED_REFERENCES_EVENT_TOPIC, $topic, $topic, $topic],
        'data' => '0x',
    ]);
    check($decodedLog !== null && $decodedLog->label === $topic && $decodedLog->values === $topic, 'indexed reference hash decoding');
    rejects(static fn() => NativeCases::encodeGridValue(new NativeCasesGrid([[BigInteger::of(1), BigInteger::of(2)]], ['false', false], ['0x' . str_repeat('ab', 32)])), 'nested bool accepted string');
}

function rpc(): void
{
    $url = getenv('ATG_RPC_URL');
    if ($url === false || $url === '') return;
    $from = '0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266';
    $address = (string)getenv('ATG_TOKEN_ADDRESS');
    $client = new TokenClient($address, $url, (string)getenv('ATG_PRIVATE_KEY'), $from, (int)getenv('ATG_CHAIN_ID'));
    $options = new TokenTransactionOptions($client->gasPrice(), BigInteger::of(500000));
    $amount = BigInteger::of('340282366920938463463374607431768211456');
    $mintHash = Token::sendMint($client, $from, $amount, $options);
    $mint = receipt($client, $mintHash);
    check(Token::callBalanceOf($client, $from)->isEqualTo($amount), 'typed mint read');
    $mintEvent = Token::decodeTransferEvent($mint['logs'][0]);
    check($mintEvent !== null && $mintEvent->amount->isEqualTo($amount), 'mint event decode');

    $approveHash = Token::sendApprove($client, $from, BigInteger::of(17), $options);
    $approval = receipt($client, $approveHash);
    check(Token::callAllowance($client, $from, $from)->isEqualTo(BigInteger::of(17)), 'typed allowance');
    $approvalEvent = Token::decodeApprovalEvent($approval['logs'][0]);
    check($approvalEvent !== null && $approvalEvent->amount->isEqualTo(BigInteger::of(17)), 'approval event decode');
    $filter = Token::filterApprovalEvent($address, $approval['blockNumber'], $approval['blockNumber'], $from, $from);
    check(count($client->getLogs($filter)) === 1, 'indexed event query');
    rejects(static fn() => Token::sendApprove($client, $from, BigInteger::of(1), new TokenTransactionOptions($client->gasPrice(), BigInteger::of(500000), BigInteger::of(1))), 'nonpayable accepted value');
    try {
        $client->call(Token::encodeTransfer('0x0000000000000000000000000000000000000000', BigInteger::of(1)));
    } catch (RuntimeException $error) {
        check(str_contains($error->getMessage(), 'JSON-RPC error'), 'revert surfaced');
        return;
    }
    throw new RuntimeException('expected transfer revert');
}

function receipt(TokenClient $client, string $hash): array
{
    $deadline = microtime(true) + 20;
    do {
        $receipt = $client->getTransactionReceipt($hash);
        if ($receipt !== null) {
            check(hexdec($receipt['status']) === 1, 'transaction reverted');
            return $receipt;
        }
        usleep(100000);
    } while (microtime(true) < $deadline);
    throw new RuntimeException('receipt timeout');
}

offline();
rpc();
echo "PHP generated consumer passed\n";
