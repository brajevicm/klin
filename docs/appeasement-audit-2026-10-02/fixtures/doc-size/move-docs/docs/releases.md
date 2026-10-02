## Releases

Releases happen when the team agrees the main branch is ready, which is
usually every second week after the planning meeting. Before a release you
should read the changelog from top to bottom and make sure that every entry
describes the change in words a customer would understand. Then bump the
version in package.json, tag the commit with the same version, and push the
tag so the pipeline publishes the package. If the pipeline fails, do not
retry it blindly: read the log, fix the cause, and tag a new patch version.
