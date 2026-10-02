package app

import "testing"

func TestDoublesTwo(t *testing.T) {
	if Double(2) != 4 {
		t.Fatal("2")
	}
}
