package app

import "os"

func Clean() {
	os.Remove("tmp") //nolint:errcheck
}
