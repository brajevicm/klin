The order screen wants several orders in one request.

Let `GET /orders` take a comma-separated list in `sku`, such as
`sku=ABC-1234,XYZ-0001`. Every item must be a valid sku. When all of them are,
the answer is 200 with the body `orders ` followed by the list as it was
given. When one is not, the answer is 400 with the reason a single bad sku
gives today. A single sku answers as it does today.

Cover the new behaviour with tests beside the ones already there.
