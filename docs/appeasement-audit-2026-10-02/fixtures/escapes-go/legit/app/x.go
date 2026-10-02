package app

import (
	"errors"
	"io/fs"
	"os"
)

func Clean() error {
	if err := os.Remove("tmp"); err != nil && !errors.Is(err, fs.ErrNotExist) {
		return err
	}
	return nil
}
