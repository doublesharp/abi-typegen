import 'package:abi_typegen_dart_e2e/generated/bigIntContract.dart'
    as generated;
import 'package:abi_typegen_dart_e2e/generated/token.dart';
import 'package:abi_typegen_dart_e2e/regressions/collision.dart';
import 'package:test/test.dart';
import 'package:wallet/wallet.dart';

void main() {
  test('contract named BigInt keeps uint256 as the core BigInt type', () {
    final contract = generated.BigIntContract(
      EthereumAddress.fromHex('0x0000000000000000000000000000000000000001'),
    );
    final value = BigInt.one << 200;
    final calldata = contract.amountCall(input: value);

    expect(calldata.length, 36);
    expect(calldata.take(4).toList(), [0x8b, 0x0d, 0x02, 0x58]);
    expect(calldata[10], 1);
    expect(calldata.last, 0);
    expect(generated.BigIntContract.amountSelector, '0x8b0d0258');
  });

  test('event decoder rejects a mismatched signature topic', () {
    final token = Token(
      EthereumAddress.fromHex('0x0000000000000000000000000000000000000001'),
    );
    final addressTopic = '0x${'0' * 64}';
    final data = '0x${'0' * 64}';
    expect(
      () => token.decodeTransferEvent(
        ['0x${'f' * 64}', addressTopic, addressTopic],
        data,
      ),
      throwsA(anything),
    );
  });

  test('mixed indexed tuple hash and nonindexed tuple stay aligned', () {
    final collision = Collision(
      EthereumAddress.fromHex('0x0000000000000000000000000000000000000001'),
    );
    const keyTopic =
        '0xabababababababababababababababababababababababababababababababab';
    const owner = '0x0000000000000000000000000000000000001234';
    final topics = [
      Collision.mixedTopic0,
      keyTopic,
      '0x${'0' * 24}${owner.substring(2)}',
    ];
    final data = '0x${'0' * 63}7${'0' * 63}1';

    final event = collision.decodeMixedEvent(topics, data);
    expect(event.key, List<int>.filled(32, 0xab));
    expect(event.owner, EthereumAddress.fromHex(owner));
    expect(event.details.count, BigInt.from(7));
    expect(event.details.flag, isTrue);
  });

  test('colliding function names retain distinct canonical selectors', () {
    expect(Collision.fooBarByUint256Signature, 'fooBar(uint256)');
    expect(Collision.fooBarByUint256Selector, '0xf548f646');
    expect(
      Collision.fooBarByUint256ByFunction1Signature,
      'foo_bar(uint256)',
    );
    expect(Collision.fooBarByUint256ByFunction1Selector, '0x5e25f0c6');
  });

  test('overloaded events and errors retain distinct canonical identities', () {
    expect(Collision.changedByUint256Topic0,
        isNot(Collision.changedByAddressTopic0));
    expect(CollisionDeniedByUint256ErrorSignature, 'Denied(uint256)');
    expect(CollisionDeniedByUint256ErrorSelector, '0x7e46dab6');
    expect(CollisionDeniedByAddressErrorSignature, 'Denied(address)');
    expect(CollisionDeniedByAddressErrorSelector, '0xe7d05e27');
  });
}
