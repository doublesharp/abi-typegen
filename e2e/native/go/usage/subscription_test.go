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
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
)

func TestGeneratedBindingsAnvilSubscription(t *testing.T) {
	httpURL := os.Getenv("ATG_RPC_URL")
	if httpURL == "" {
		t.Skip("run through e2e/native/anvil.py for isolated RPC coverage")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	wsURL := strings.Replace(httpURL, "http://", "ws://", 1)
	client, err := ethclient.DialContext(ctx, wsURL)
	if err != nil {
		t.Fatal(err)
	}
	defer client.Close()
	binding, err := contracts.NewTokenBinding(common.HexToAddress(os.Getenv("ATG_TOKEN_ADDRESS")), client)
	if err != nil {
		t.Fatal(err)
	}
	key, err := crypto.HexToECDSA(strings.TrimPrefix(os.Getenv("ATG_PRIVATE_KEY"), "0x"))
	if err != nil {
		t.Fatal(err)
	}
	chainID, err := client.ChainID(ctx)
	if err != nil {
		t.Fatal(err)
	}
	opts, err := bind.NewKeyedTransactorWithChainID(key, chainID)
	if err != nil {
		t.Fatal(err)
	}
	opts.Context = ctx
	events, subscription, err := binding.WatchApproval(ctx, []any{opts.From}, []any{opts.From})
	if err != nil {
		t.Fatal(err)
	}
	defer subscription.Unsubscribe()
	tx, err := binding.Approve(opts, contracts.TokenApproveParams{Spender: opts.From, Amount: big.NewInt(17)})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := bind.WaitMined(ctx, client, tx); err != nil {
		t.Fatal(err)
	}
	select {
	case event, ok := <-events:
		if !ok {
			t.Fatal("subscription closed before approval event")
		}
		if event.Owner != opts.From || event.Spender != opts.From || event.Amount.Cmp(big.NewInt(17)) != 0 {
			t.Fatalf("incorrect subscribed event: %#v", event)
		}
	case err := <-subscription.Err():
		t.Fatalf("subscription ended: %v", err)
	case <-ctx.Done():
		t.Fatal("timed out waiting for approval subscription")
	}
}
