package usage

import (
	"math/big"
	"testing"

	"github.com/doublesharp/abi-typegen/e2e/native/go/usage/tupleindexed"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
)

func TestIndexedTupleEventPreservesTopicHash(t *testing.T) {
	binding, err := tupleindexed.NewIndexedTupleBinding(common.Address{}, nil)
	if err != nil {
		t.Fatal(err)
	}
	definition := binding.ABI().Events["Recorded"]
	count := big.NewInt(7)
	data, err := definition.Inputs.NonIndexed().Pack(count)
	if err != nil {
		t.Fatal(err)
	}
	hash := common.HexToHash("0x1234")
	owner := common.HexToAddress("0x123456")
	ownerTopic := common.BytesToHash(owner.Bytes())
	event, err := binding.DecodeRecordedEvent(types.Log{Topics: []common.Hash{definition.ID, hash, ownerTopic}, Data: data})
	if err != nil {
		t.Fatal(err)
	}
	if event.Item != hash || event.Owner != owner || event.Count.Cmp(count) != 0 {
		t.Fatalf("indexed tuple event mismatch: %#v", event)
	}
	if _, err := binding.DecodeRecordedEvent(types.Log{Topics: []common.Hash{definition.ID, hash}, Data: data}); err == nil {
		t.Fatal("missing indexed topic accepted")
	}
}
