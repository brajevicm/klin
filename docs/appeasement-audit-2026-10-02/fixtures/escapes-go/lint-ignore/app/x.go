package app

import "os"

func Clean() {
	//lint:ignore errcheck the file may be gone
	os.Remove("tmp")
}
