package usage

import (
	"encoding/hex"
	"math/big"
	"testing"

	"github.com/doublesharp/abi-typegen/e2e/native/go/contracts"
	"github.com/ethereum/go-ethereum/common"
)

func TestOverloadAliases(t *testing.T) {
	c, err := contracts.NewOverloadsBinding(common.Address{}, nil)
	if err != nil {
		t.Fatal(err)
	}
	check := func(signature, wantSignature, selector string, data []byte, err error) {
		t.Helper()
		if err != nil {
			t.Fatal(err)
		}
		if signature != wantSignature || len(data) < 4 || hex.EncodeToString(data[:4]) != selector {
			t.Fatalf("%s: %x; want %s: %s", signature, data, wantSignature, selector)
		}
	}
	data, err := c.EncodeSafeTransferFrom(contracts.OverloadsSafeTransferFromParams{TokenId: big.NewInt(7)})
	check(contracts.OverloadsSafeTransferFromSignature, "safeTransferFrom(address,address,uint256)", "42842e0e", data, err)
	data, err = c.EncodeSafeTransferFrom0(contracts.OverloadsSafeTransferFrom0Params{TokenId: big.NewInt(7), Data: []byte{1}})
	check(contracts.OverloadsSafeTransferFrom0Signature, "safeTransferFrom(address,address,uint256,bytes)", "b88d4fde", data, err)
	data, err = c.EncodeAttestations(contracts.OverloadsAttestationsParams{})
	check(contracts.OverloadsAttestationsSignature, "attestations(bytes32,bytes32)", "92abd17b", data, err)
	data, err = c.EncodeAttestations0(contracts.OverloadsAttestations0Params{})
	check(contracts.OverloadsAttestations0Signature, "attestations(bytes32)", "940992a3", data, err)
	data, err = c.EncodeSetAddr(contracts.OverloadsSetAddrParams{CoinType: big.NewInt(60), Value: []byte{2}})
	check(contracts.OverloadsSetAddrSignature, "setAddr(bytes32,uint256,bytes)", "8b95dd71", data, err)
	data, err = c.EncodeSetAddr0(contracts.OverloadsSetAddr0Params{})
	check(contracts.OverloadsSetAddr0Signature, "setAddr(bytes32,address)", "d5fa2b00", data, err)
}
