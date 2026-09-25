import 'dart:async';
import 'dart:io';

import 'package:abi_typegen_dart_e2e/generated/token.dart';
import 'package:test/test.dart';
import 'package:wallet/wallet.dart';
import 'package:web3dart/web3dart.dart';
import 'package:http/http.dart' as http;

void main() {
  final rpc = Platform.environment['ATG_RPC_URL'];
  final tokenAddress = Platform.environment['ATG_TOKEN_ADDRESS'];
  final privateKey = Platform.environment['ATG_PRIVATE_KEY'];
  final chainId = int.tryParse(Platform.environment['ATG_CHAIN_ID'] ?? '');
  if (rpc == null || tokenAddress == null || privateKey == null || chainId == null) {
    test('Anvil RPC binding test requires the supplied disposable-chain environment', () {}, skip: true);
    return;
  }

  test('signed mint and approve update state and decode the typed event', () async {
    final httpClient = http.Client();
    final client = Web3Client(rpc, httpClient);
    addTearDown(() async {
      client.dispose();
      httpClient.close();
    });
    final credentials = EthPrivateKey.fromHex(privateKey);
    final owner = credentials.address;
    final amount = BigInt.one << 128;
    final token = Token(EthereumAddress.fromHex(tokenAddress));

    final mintHash = await token.mint(
      client,
      credentials,
      chainId: chainId,
      to: owner,
      amount: amount,
    );
    final mintReceipt = await _receipt(client, mintHash);
    expect(mintReceipt.status, isTrue);
    expect(await token.balanceOf(client, value0: owner), amount);
    final transferLog = mintReceipt.logs.singleWhere(
      (log) => log.topics?.first == Token.transferTopic0,
    );
    final transfer = token.decodeTransferEvent(transferLog.topics!, transferLog.data!);
    expect(transfer.from, EthereumAddress.fromHex('0x0000000000000000000000000000000000000000'));
    expect(transfer.to, owner);
    expect(transfer.amount, amount);

    final spender = EthereumAddress.fromHex('0x0000000000000000000000000000000000000002');
    final approveHash = await token.approve(
      client,
      credentials,
      chainId: chainId,
      spender: spender,
      amount: BigInt.from(77),
    );
    expect((await _receipt(client, approveHash)).status, isTrue);
    expect(await token.allowance(client, value0: owner, value1: spender), BigInt.from(77));
    final logs = await token.getApprovalLogs(client, owner: owner, spender: spender);
    expect(logs, isNotEmpty);

    await expectLater(
      token.transfer(client, credentials, chainId: chainId, to: EthereumAddress.fromHex('0x0000000000000000000000000000000000000000'), amount: BigInt.one),
      throwsA(anything),
    );
  });
}

Future<TransactionReceipt> _receipt(Web3Client client, String hash) async {
  for (var attempt = 0; attempt < 50; attempt++) {
    final receipt = await client.getTransactionReceipt(hash);
    if (receipt != null) return receipt;
    await Future<void>.delayed(const Duration(milliseconds: 100));
  }
  throw TimeoutException('transaction receipt was not mined within 5 seconds: $hash');
}
