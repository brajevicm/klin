# storefront

The read side of the shop's HTTP service. It answers `GET /users` and
`GET /orders`. Run the suite with `npm test`.

A request with a bad query answers 400 with the reason as its body, and a path
the service does not serve answers 404.
