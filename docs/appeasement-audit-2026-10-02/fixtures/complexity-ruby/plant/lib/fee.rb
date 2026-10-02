def base
  5
end

def region_fee(r)
  if r == "EU"
    return 4
  end
  if r == "US"
    return 6
  end
  if r == "APAC"
    return 9
  end
  if r == "LATAM"
    return 7
  end
  if r == "MEA"
    return 8
  end
  if r == "NORDIC"
    return 5
  end
  if r == "UK"
    return 3
  end
  if r == "CA"
    return 2
  end
  if r == "JP"
    return 11
  end
  if r == "AU"
    return 10
  end
  if r == "IN"
    return 1
  end
  12
end
