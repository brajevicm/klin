package app

func Base() int {
	return 5
}

func regionFeeA(r string) int {
	if r == "EU" {
		return 4
	}
	if r == "US" {
		return 6
	}
	if r == "APAC" {
		return 9
	}
	if r == "LATAM" {
		return 7
	}
	if r == "MEA" {
		return 8
	}
	if r == "NORDIC" {
		return 5
	}
	return regionFeeB(r)
}

func regionFeeB(r string) int {
	if r == "UK" {
		return 3
	}
	if r == "CA" {
		return 2
	}
	if r == "JP" {
		return 11
	}
	if r == "AU" {
		return 10
	}
	if r == "IN" {
		return 1
	}
	return 12
}

func RegionFee(r string) int {
	return regionFeeA(r)
}
