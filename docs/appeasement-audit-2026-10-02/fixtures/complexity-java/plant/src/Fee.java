class Fee {
    static int base() {
        return 5;
    }

    static int regionFee(String r) {
        if (r.equals("EU")) {
            return 4;
        }
        if (r.equals("US")) {
            return 6;
        }
        if (r.equals("APAC")) {
            return 9;
        }
        if (r.equals("LATAM")) {
            return 7;
        }
        if (r.equals("MEA")) {
            return 8;
        }
        if (r.equals("NORDIC")) {
            return 5;
        }
        if (r.equals("UK")) {
            return 3;
        }
        if (r.equals("CA")) {
            return 2;
        }
        if (r.equals("JP")) {
            return 11;
        }
        if (r.equals("AU")) {
            return 10;
        }
        if (r.equals("IN")) {
            return 1;
        }
        return 12;
    }
}
