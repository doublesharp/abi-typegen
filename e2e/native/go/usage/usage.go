// Package usage checks that generated Go bindings compile and fit go-ethereum.
package usage

import (
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"

	"github.com/doublesharp/abi-typegen/e2e/native/go/contracts"
)

// Check parses the embedded ABI and compares it with the generated constants.
func Check() error {
	parsed, err := abi.JSON(stringsReader(contracts.TokenABI))
	if err != nil {
		return err
	}
	if parsed.Methods["transfer"].Sig != contracts.TokenTransferSignature {
		return errMismatch("transfer signature")
	}
	if [4]byte(parsed.Methods["transfer"].ID) != contracts.TokenTransferSelector {
		return errMismatch("transfer selector")
	}
	if parsed.Events["Transfer"].ID != contracts.TokenTransferEventTopic {
		return errMismatch("Transfer topic")
	}

	// Named tuple structs pack through go-ethereum's ABI encoder, and
	// overloads use the same names go-ethereum gives them.
	tuples, err := abi.JSON(stringsReader(contracts.TupleCasesABI))
	if err != nil {
		return err
	}
	if tuples.Methods["deposit0"].Sig != contracts.TupleCasesDeposit0Signature {
		return errMismatch("overload naming")
	}
	account := contracts.TupleCasesTupleAccountPosition{Account: common.HexToAddress("0x01")}
	if _, err := tuples.Pack("deposit", account); err != nil {
		return err
	}

	// Output tuples decode into the generated struct.
	vault, err := abi.JSON(stringsReader(contracts.VaultABI))
	if err != nil {
		return err
	}
	want := contracts.VaultPosition{Shares: big.NewInt(7), DepositedAt: 9, Token: common.HexToAddress("0x02")}
	encoded, err := vault.Methods["getPosition"].Outputs.Pack(want)
	if err != nil {
		return err
	}
	values, err := vault.Unpack("getPosition", encoded)
	if err != nil {
		return err
	}
	got := *abi.ConvertType(values[0], new(contracts.VaultPosition)).(*contracts.VaultPosition)
	if got.Shares.Cmp(want.Shares) != 0 || got.DepositedAt != want.DepositedAt || got.Token != want.Token {
		return errMismatch("decoded tuple")
	}

	// Nested tuples with arrays, fixed arrays, and bytes round-trip.
	native, err := abi.JSON(stringsReader(contracts.NativeCasesABI))
	if err != nil {
		return err
	}
	batch := contracts.NativeCasesBatch{
		Calls: []contracts.NativeCasesCall3Value{{
			Target: common.HexToAddress("0x03"), AllowFailure: true, Value: big.NewInt(10), CallData: []byte{1, 2},
		}},
		Grid: contracts.NativeCasesGrid{
			Rows: [][]*big.Int{{big.NewInt(1)}, {}}, Flags: [2]bool{true, false}, Tags: [][32]byte{{7}},
		},
		Fee:   big.NewInt(3000),
		Delta: big.NewInt(-5),
	}
	packed, err := native.Pack("aggregate", batch)
	if err != nil {
		return err
	}
	if [4]byte(packed[:4]) != contracts.NativeCasesAggregateSelector {
		return errMismatch("aggregate selector")
	}
	args, err := native.Methods["aggregate"].Inputs.Unpack(packed[4:])
	if err != nil {
		return err
	}
	decoded := *abi.ConvertType(args[0], new(contracts.NativeCasesBatch)).(*contracts.NativeCasesBatch)
	if decoded.Fee.Cmp(batch.Fee) != 0 || decoded.Grid.Flags != batch.Grid.Flags || decoded.Calls[0].Value.Cmp(big.NewInt(10)) != 0 {
		return errMismatch("decoded batch")
	}

	params := contracts.TokenTransferParams{To: common.Address{}, Amount: big.NewInt(1)}
	if _, err := parsed.Pack("transfer", params.To, params.Amount); err != nil {
		return err
	}
	return nil
}
