import 'dart:typed_data';

import 'package:abi_typegen_dart_e2e/regressions/numeric.dart';
import 'package:wallet/wallet.dart';

void expectRejected(void Function() action, String description) {
  try {
    action();
  } on ArgumentError {
    return;
  }
  throw StateError('generated binding accepted invalid $description');
}

void main() {
  final numeric = Numeric(EthereumAddress.fromHex('0x0000000000000000000000000000000000000001'));

  expectRejected(() => numeric.uint8ValueCall(value_: BigInt.from(256)), 'uint8');
  expectRejected(() => numeric.uint8ValueCall(value_: BigInt.from(-1)), 'negative uint8');
  expectRejected(() => numeric.int8ValueCall(value_: BigInt.from(128)), 'int8');
  expectRejected(() => numeric.int8ValueCall(value_: BigInt.from(-129)), 'negative int8');
  expectRejected(() => numeric.fixedBytesValueCall(value_: Uint8List(1)), 'bytes2');
  expectRejected(() => numeric.fixedArrayValueCall(value_: [BigInt.one]), 'uint256[2]');
  numeric.fixedArrayValueCall(value_: [BigInt.zero, BigInt.one]);
}
