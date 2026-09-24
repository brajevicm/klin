The parcel counter prints one line per parcel on the receipt.

Add `receiptLine(parcel)` to `src/rate.ts`. It returns the zone, a comma and a
space, the charged weight in whole kilograms followed by ` kg`, a comma and a
space, the service, a colon and a space, and the price with exactly two
decimal places. The charged weight is the weight rounded up to the next whole
kilogram. A 2.1 kg standard parcel to `eu` prints `eu, 3 kg, standard: 12.10`.

Cover the new behaviour with tests beside the ones already there.
