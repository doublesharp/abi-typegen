package usage

import (
	"errors"
	"io"
	"strings"
)

func stringsReader(s string) io.Reader { return strings.NewReader(s) }

func errMismatch(what string) error {
	return errors.New("generated " + what + " does not match go-ethereum")
}
