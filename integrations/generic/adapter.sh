#!/bin/sh
# A reference shim for klin's generic harness contract, version 1.
#
# It reads one generic event on stdin, runs the klin command that event belongs to, and passes
# klin's decision back on stdout with klin's exit code. It is the translation step and nothing
# else: a real integration wraps it, or replaces it, with the code that turns its own host's
# event into this shape and the decision back into its own host's answer.
#
# See ../../docs/HARNESS_INTEGRATION.md.
set -eu

klin=${KLIN:-klin}
event=$(cat)
kind=$(printf '%s' "$event" | tr -d ' \n' | sed -n 's/.*"event":"\([a-z_]*\)".*/\1/p')

case "$kind" in
  session | prompt) set -- radius ;;
  pre_tool) set -- guard ;;
  stop) set -- gate --hook --changed ;;
  *)
    echo "adapter.sh: this event names no generic event kind: '$kind'" >&2
    exit 1
    ;;
esac

printf '%s' "$event" | "$klin" "$@"
