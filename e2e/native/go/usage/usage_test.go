package usage

import (
	"testing"

	"github.com/doublesharp/abi-typegen/e2e/native/go/contracts"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

func TestCheck(t *testing.T) {
	if err := Check(); err != nil {
		t.Fatal(err)
	}
}

// Indexed reference values decode to hashes because the original values are not in the log.
func TestIndexedReferenceTopics(t *testing.T) {
	parsed, err := abi.JSON(stringsReader(contracts.NativeCasesABI))
	if err != nil {
		t.Fatal(err)
	}
	topics := []common.Hash{common.HexToHash("0x01"), common.HexToHash("0x02"), common.HexToHash("0x03")}
	var event contracts.NativeCasesIndexedReferencesEvent
	if err := abi.ParseTopics(&event, parsed.Events["IndexedReferences"].Inputs, topics); err != nil {
		t.Fatal(err)
	}
	if event.Label != topics[0] || event.Payload != topics[1] || event.Values != topics[2] {
		t.Fatal("indexed reference fields must preserve their topic hashes")
	}
	var fixed contracts.NativeCasesIndexedFixedEvent
	if err := abi.ParseTopics(&fixed, parsed.Events["IndexedFixed"].Inputs, topics[:1]); err != nil {
		t.Fatal(err)
	}
	if fixed.Pair != topics[0] {
		t.Fatal("indexed fixed array must preserve its topic hash")
	}
}
