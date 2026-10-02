def base
  5
end

REGION = {"EU" => 4, "US" => 6, "APAC" => 9, "LATAM" => 7, "MEA" => 8, "NORDIC" => 5, "UK" => 3, "CA" => 2, "JP" => 11, "AU" => 10, "IN" => 1}.freeze

def region_fee(r)
  REGION.fetch(r, 12)
end
