package app

func Base() int {
	return 5
}

var regionFees = map[string]int{"EU": 4, "US": 6, "APAC": 9, "LATAM": 7, "MEA": 8, "NORDIC": 5, "UK": 3, "CA": 2, "JP": 11, "AU": 10, "IN": 1}

func RegionFee(r string) int {
	if fee, ok := regionFees[r]; ok {
		return fee
	}
	return 12
}
