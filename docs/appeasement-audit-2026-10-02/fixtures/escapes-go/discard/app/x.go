package app

import "os"

func Clean() {
	_ = os.Remove("tmp")
}
