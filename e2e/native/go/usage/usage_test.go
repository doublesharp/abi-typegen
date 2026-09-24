package usage

import "testing"

func TestCheck(t *testing.T) {
	if err := Check(); err != nil {
		t.Fatal(err)
	}
}
