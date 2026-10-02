func base() -> Int {
    return 5
}

private let region: [String: Int] = ["EU": 4, "US": 6, "APAC": 9, "LATAM": 7, "MEA": 8, "NORDIC": 5, "UK": 3, "CA": 2, "JP": 11, "AU": 10, "IN": 1]

func regionFee(_ r: String) -> Int {
    return region[r] ?? 12
}
