package app

import "strings"

func Export() string {
	return strings.Join([]string{"name", "total"}, ",")
}
