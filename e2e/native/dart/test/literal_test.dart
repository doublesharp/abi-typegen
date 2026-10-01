import 'dart:convert';

import 'package:abi_typegen_dart_e2e/regressions/dartLiteralCase.dart';
import 'package:abi_typegen_dart_e2e/regressions/dollarToken.dart';
import 'package:test/test.dart';
import 'package:wallet/wallet.dart';

void main() {
  test('dollar contract names and Object member fields stay usable', () {
    expect(DollarTokenAbi.name, 'Dollar\$Token');
    final result = DollarTokenReadFunctionResult(
      runtimeType_: BigInt.from(7),
      hashCode_: true,
      toString_: 'value',
      runtimeType_2: false,
    );
    expect(result.runtimeType_, BigInt.from(7));
    expect(result.hashCode_, isTrue);
    expect(result.toString_, 'value');
    expect(result.toString(), isA<String>());
    final contract = DollarToken(
      EthereumAddress.fromHex('0x0000000000000000000000000000000000000001'),
    );
    String word(int value) => value.toRadixString(16).padLeft(64, '0');
    final data = '0x${word(7)}${word(1)}${word(128)}${word(0)}'
        '${word(5)}${'76616c7565'.padRight(64, '0')}';
    final decoded = contract.decodeReadResult(data);
    expect(decoded.runtimeType_, BigInt.from(7));
    expect(decoded.hashCode_, isTrue);
    expect(decoded.toString_, 'value');
    expect(decoded.runtimeType_2, isFalse);
  });
  test('ABI literals preserve quotes, interpolation markers, and backslashes',
      () {
    final entries = jsonDecode(DartLiteralCaseAbiJson) as List<dynamic>;
    const expected = "quote ''' dollar \$value backslash \\ newline\n";
    expect(entries[0]['extra'], expected);
    expect(entries[1]['extra'], expected);

    final contract = DartLiteralCase(
      EthereumAddress.fromHex('0x0000000000000000000000000000000000000001'),
    );
    expect(contract.readCall(), hasLength(4));
    expect(
      contract.decodeDeniedError(DartLiteralCaseDeniedErrorSelector),
      isA<DartLiteralCaseErrorDenied>(),
    );
    expect(DartLiteralCase.fooBarSignature, 'foo\$bar(uint8)');
    expect(contract.fooBarCall(xY: BigInt.from(7)), hasLength(36));
    expect(() => contract.fooBarCall(xY: BigInt.from(256)), throwsRangeError);
    expect(DartLiteralCaseDeniedNowErrorSignature, 'Denied\$Now()');
    expect(
      contract.decodeDeniedNowError(DartLiteralCaseDeniedNowErrorSelector),
      isA<DartLiteralCaseErrorDeniedNow>(),
    );
    expect(contract.changedNowEvent.stringSignature, 'Changed\$Now()');
    expect(
      contract.decodeChangedNowEvent([DartLiteralCase.changedNowTopic0], '0x'),
      isA<DartLiteralCaseChangedNowEvent>(),
    );
    expect(DartLiteralCaseNestedDeniedErrorSignature,
        'NestedDenied((int8,bool)[])');
    final nestedPayload = '$DartLiteralCaseNestedDeniedErrorSelector'
        '${'20'.padLeft(64, '0')}${'1'.padLeft(64, '0')}'
        '${'f' * 64}${'1'.padLeft(64, '0')}';
    final nested = contract.decodeNestedDeniedError(nestedPayload);
    expect(nested.items.single.count, BigInt.from(-1));
    expect(nested.items.single.ok, isTrue);
  });
}
