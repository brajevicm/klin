package app

import "testing"

func TestDoublesTwo(t *testing.T) {
	if Double(2) != 4 {
		t.Fatal("2")
	}
}

func testDoublesZero(t *testing.T) {
	if Double(0) != 0 {
		t.Fatal("0")
	}
}
