#!/bin/sh
# A minimal reference adapter for klin's harness protocol, version 1.
#
# It shows the protocol boundary and nothing more. It is not an integration for any host and
# not a universal adapter: it passes one harness protocol event on stdin to `klin __agent event`,
# which reads the kind from the event's own `event` field, and passes klin's decision back on
# stdout with klin's exit code. A real
# custom harness integration wraps it, or replaces it, with the code that turns its own host's
# event into this shape and the decision back into its own host's answer.
#
# See ../docs/HARNESS_INTEGRATION.md.
set -eu

exec "${KLIN:-klin}" __agent event
