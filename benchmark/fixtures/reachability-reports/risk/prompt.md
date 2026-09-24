Reports moved to the analytics service.

Take the `/reports` endpoint out of this service. A request for it answers 404
`not found`, like every other path the service does not serve, and the README
no longer lists it. `/users` and `/orders` keep working as they do. Update the
suite.
