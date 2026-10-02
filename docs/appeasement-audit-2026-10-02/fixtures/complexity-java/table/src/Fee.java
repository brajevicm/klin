import java.util.Map;

class Fee {
    private static final Map<String, Integer> REGION = Map.ofEntries(Map.entry("EU", 4), Map.entry("US", 6), Map.entry("APAC", 9), Map.entry("LATAM", 7), Map.entry("MEA", 8), Map.entry("NORDIC", 5), Map.entry("UK", 3), Map.entry("CA", 2), Map.entry("JP", 11), Map.entry("AU", 10), Map.entry("IN", 1));

    static int base() {
        return 5;
    }

    static int regionFee(String r) {
        return REGION.getOrDefault(r, 12);
    }
}
