package usage

import (
	"context"
	"math/big"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/doublesharp/abi-typegen/e2e/native/go/contracts"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
)

func TestGeneratedBindingsAnvil(t *testing.T) {
	url := os.Getenv("ATG_RPC_URL")
	if url == "" {
		t.Skip("run through e2e/native/anvil.py for isolated RPC coverage")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	rpc, err := ethclient.DialContext(ctx, url)
	if err != nil {
		t.Fatal(err)
	}
	defer rpc.Close()
	chainID, err := rpc.ChainID(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if chainID.String() != os.Getenv("ATG_CHAIN_ID") {
		t.Fatal("unexpected chain")
	}
	key, err := crypto.HexToECDSA(strings.TrimPrefix(os.Getenv("ATG_PRIVATE_KEY"), "0x"))
	if err != nil {
		t.Fatal(err)
	}
	opts, err := bind.NewKeyedTransactorWithChainID(key, chainID)
	if err != nil {
		t.Fatal(err)
	}
	opts.Context = ctx
	address := common.HexToAddress(os.Getenv("ATG_TOKEN_ADDRESS"))
	token, err := contracts.NewTokenBinding(address, rpc)
	if err != nil {
		t.Fatal(err)
	}
	amount := new(big.Int).Lsh(big.NewInt(1), 128)
	tx, err := token.Mint(opts, contracts.TokenMintParams{To: opts.From, Amount: amount})
	if err != nil {
		t.Fatal(err)
	}
	receipt, err := bind.WaitMined(ctx, rpc, tx)
	if err != nil {
		t.Fatal(err)
	}
	if receipt.Status != types.ReceiptStatusSuccessful {
		t.Fatal("transaction reverted")
	}
	balance, err := token.BalanceOf(&bind.CallOpts{Context: ctx}, contracts.TokenBalanceOfParams{Address: opts.From})
	if err != nil {
		t.Fatal(err)
	}
	if balance.Uint256.Cmp(amount) != 0 {
		t.Fatalf("balance %s != %s", balance.Uint256, amount)
	}
	if len(receipt.Logs) != 1 {
		t.Fatal("missing mint event")
	}
	event, err := token.DecodeTransferEvent(*receipt.Logs[0])
	if err != nil {
		t.Fatal(err)
	}
	if event.To != opts.From || event.Amount.Cmp(amount) != 0 {
		t.Fatalf("incorrect decoded event: %#v", event)
	}
	events, err := token.FilterTransfer(ctx, receipt.BlockNumber, receipt.BlockNumber)
	if err != nil || len(events) != 1 {
		t.Fatalf("event filter: %v, %d logs", err, len(events))
	}
}
