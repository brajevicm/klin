The parcel counter now sends to the Channel Islands and the Canaries.

Add a zone called `islands` to `src/rate.ts`. The first kilogram costs 12
euros and each started kilogram after it costs 2.40. The heavy-parcel
surcharge and the fuel surcharge apply the same way as for every zone outside
the country. The carrier offers no express service to the islands, so an
express parcel to that zone is refused like an unknown zone.

Cover the new zone with tests beside the ones already there.
