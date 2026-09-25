import 'package:abi_typegen_dart_e2e/generated/token.dart';
import 'package:test/test.dart';
import 'package:wallet/wallet.dart';
import 'package:web3dart/web3dart.dart';

void main() {
  final token = Token(
      EthereumAddress.fromHex('0x0000000000000000000000000000000000000001'));

  test('offline function encoding uses the canonical selector and typed values',
      () {
    final call = token.transferCall(
      to: EthereumAddress.fromHex('0x0000000000000000000000000000000000000002'),
      amount: BigInt.from(17),
    );
    expect(call.take(4).toList(), [0xa9, 0x05, 0x9c, 0xbb]);
    expect(Token.transferSelector, '0xa9059cbb');
  });

  test('offline return decoder produces the declared Solidity output type', () {
    expect(token.decodeTransferResult('0x${'0' * 63}1'), isTrue);
    expect(() => token.decodeTransferResult('0x'), throwsA(anything));
  });

  test('event decoding keeps typed indexed values and event topic hash', () {
    final from = '0x0000000000000000000000000000000000000003';
    final to = '0x0000000000000000000000000000000000000004';
    final topics = [
      bytesToHex(token.transferEvent.signature, include0x: true),
      '0x${'0' * 24}${from.substring(2)}',
      '0x${'0' * 24}${to.substring(2)}',
    ];
    final event = token.decodeTransferEvent(topics, '0x${'0' * 63}9');
    expect(event.from, EthereumAddress.fromHex(from));
    expect(event.to, EthereumAddress.fromHex(to));
    expect(event.amount, BigInt.from(9));
    expect(Token.transferTopic0, topics.first);
  });

  test(
      'declared custom errors decode typed arguments and reject a mismatched selector',
      () {
    final payload = '$TokenInsufficientBalanceErrorSelector'
        '${'0' * 24}${'1' * 40}'
        '${'0' * 63}a'
        '${'0' * 63}b';
    final error = token.decodeInsufficientBalanceError(payload);
    expect(error.account, EthereumAddress.fromHex('0x${'1' * 40}'));
    expect(error.available, BigInt.from(10));
    expect(error.required_, BigInt.from(11));
    expect(() => token.decodeInsufficientBalanceError('0x00000000'),
        throwsFormatException);
    expect(
        token.decodeInvalidRecipientError(TokenInvalidRecipientErrorSelector),
        isA<TokenErrorInvalidRecipient>());
  });
}
