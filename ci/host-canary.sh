#!/usr/bin/env bash
# One golden lifecycle per host, run against the current stable host release.
# Usage: ci/host-canary.sh claude|codex|cursor
# Classification goes to $ART/classification.txt as PASS, COMPAT, INFRA or INCONCLUSIVE.
set -u

HOST="${1:?usage: host-canary.sh claude|codex|cursor}"
CHECKOUT="${CHECKOUT:-$(cd "$(dirname "$0")/.." && pwd)}"
ART="${ART:-$(mktemp -d)/$HOST}"
PROMPT='Write README.md so that it holds the word "word" 200 times, separated by spaces. Change no other file.'

mkdir -p "$ART"
export HOME="${CANARY_HOME:-$(mktemp -d)}"

say() { printf '%s\n' "$*" | tee -a "$ART/canary.log"; }

verdict() {
  printf '%s\n' "$1" > "$ART/classification.txt"
  printf '%s: %s\n' "$1" "$2" | tee -a "$ART/canary.log"
  if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    printf '### %s canary: **%s**\n\n%s\n' "$HOST" "$1" "$2" >> "$GITHUB_STEP_SUMMARY"
  fi
  case "$1" in
    COMPAT) exit 1 ;;
    *) exit 0 ;;
  esac
}

run() {
  say "+ $*"
  "$@" >> "$ART/host.log" 2>&1
}

# A disposable repository whose only gate failure is the one the turn creates.
repo() {
  REPO="$(mktemp -d)/canary"
  mkdir -p "$REPO"
  cd "$REPO" || verdict INFRA "the disposable repository could not be created"
  git init -q . || verdict INFRA "the disposable repository could not be initialized"
  git config user.email canary@klin.invalid
  git config user.name "klin canary"
  git config commit.gpgsign false
  printf '{"doc_size": {"README.md": 10}}\n' > klin.json
  printf 'word word word\n' > README.md
  git add -A
  git commit -qm "base" || verdict INFRA "the disposable repository has no base commit, so klin has nothing to ratchet against"
}

install_host() {
  case "$HOST" in
    claude)
      run npm install -g @anthropic-ai/claude-code@latest || verdict INFRA "the Claude Code package could not be installed"
      command -v claude > /dev/null || verdict INFRA "claude is not on PATH after the install"
      claude --version > "$ART/host-version.txt" 2>&1
      ;;
    codex)
      run npm install -g @openai/codex@latest || verdict INFRA "the Codex package could not be installed"
      command -v codex > /dev/null || verdict INFRA "codex is not on PATH after the install"
      codex --version > "$ART/host-version.txt" 2>&1
      ;;
    cursor)
      run bash -c 'set -o pipefail; curl https://cursor.com/install -fsS | bash' || verdict INFRA "the Cursor agent could not be installed"
      PATH="$HOME/.local/bin:$PATH"
      export PATH
      command -v cursor-agent > /dev/null || verdict INCONCLUSIVE "Cursor exposes no headless agent on this runner; the Cursor plugin route is release-smoke-only"
      cursor-agent --version > "$ART/host-version.txt" 2>&1
      ;;
  esac
  say "host version: $(cat "$ART/host-version.txt")"
}

install_failed() {
  local url tag
  url=$(jq -r '.plugins[0].source.url' "$CHECKOUT/$1")
  tag=$(jq -r '.plugins[0].source.ref' "$CHECKOUT/$1")
  run env GIT_TERMINAL_PROMPT=0 git ls-remote --exit-code "$url" "refs/tags/$tag"
  case $? in
    0) verdict COMPAT "$2" ;;
    2) verdict INFRA "klin's marketplace names $tag, which $url does not hold, so no host can install the plugin" ;;
    *) verdict INFRA "klin's plugin source $url could not be fetched at $tag, so the host never saw the plugin" ;;
  esac
}

install_plugin() {
  case "$HOST" in
    claude)
      run claude plugin marketplace add "$CHECKOUT" || verdict COMPAT "the current Claude Code rejected klin's marketplace manifest"
      run claude plugin install klin@klin || install_failed .claude-plugin/marketplace.json "the current Claude Code rejected the klin plugin"
      claude plugin list --json > "$ART/plugins.json" 2>&1
      grep -q '"klin"' "$ART/plugins.json" || verdict COMPAT "the current Claude Code does not list klin after a successful install"
      ;;
    codex)
      run codex plugin marketplace add "$CHECKOUT" || verdict COMPAT "the current Codex rejected klin's marketplace manifest"
      run codex plugin add klin@klin || install_failed .agents/plugins/marketplace.json "the current Codex rejected the klin plugin"
      # Codex does not trust plugin hooks on install. In CI only, the trust step is
      # supplied by configuration, because no headless trust flow is documented.
      if [ -n "${CODEX_TRUST_COMMAND:-}" ]; then
        run bash -c "$CODEX_TRUST_COMMAND" || verdict INCONCLUSIVE "the configured Codex hook-trust step failed"
      else
        verdict INCONCLUSIVE "no Codex hook-trust step is configured, so trusted hooks cannot run headlessly"
      fi
      ;;
    cursor)
      mkdir -p "$HOME/.cursor/plugins/local"
      cp -R "$CHECKOUT/plugins/klin" "$HOME/.cursor/plugins/local/klin" || verdict INFRA "the Cursor plugin copy failed"
      # The local plugin directory is the editor's route. No documented Cursor surface says
      # the headless agent loads it, so this row stays release-smoke-only until one does,
      # and CURSOR_HEADLESS_PLUGINS=1 says somebody verified that route by hand.
      [ "${CURSOR_HEADLESS_PLUGINS:-0}" = "1" ] ||
        verdict INCONCLUSIVE "Cursor documents no headless route that loads ~/.cursor/plugins/local, so the plugin install stays release-smoke-only"
      ;;
  esac
}

session() {
  case "$HOST" in
    claude) [ -n "${ANTHROPIC_API_KEY:-}" ] || verdict INCONCLUSIVE "no Claude Code credential is configured, so no session can run" ;;
    codex) [ -n "${OPENAI_API_KEY:-}" ] || verdict INCONCLUSIVE "no Codex credential is configured, so no session can run" ;;
    cursor) [ -n "${CURSOR_API_KEY:-}" ] || verdict INCONCLUSIVE "no Cursor credential is configured, so no session can run" ;;
  esac
  case "$HOST" in
    claude) claude -p "$PROMPT" --debug --permission-mode acceptEdits > "$ART/session.log" 2>&1 ;;
    codex) codex exec "$PROMPT" > "$ART/session.log" 2>&1 ;;
    cursor) cursor-agent -p "$PROMPT" > "$ART/session.log" 2>&1 ;;
  esac
  CODE=$?
  say "session exit: $CODE"
}

evidence() {
  JOURNAL="$REPO/.git/klin/journal.jsonl"
  git -C "$REPO" status --porcelain -- README.md > "$ART/tree.txt"
  git -C "$REPO" status --porcelain > "$ART/tree-all.txt"
  [ -f "$JOURNAL" ] && cp "$JOURNAL" "$ART/journal.jsonl"

  if [ ! -f "$JOURNAL" ]; then
    grep -q 'klin is not installed' "$ART/session.log" &&
      verdict INFRA "the plugin wrapper reached no klin binary, so the pinned release download failed"
    grep -qi 'auth\|credential\|login\|quota\|rate limit' "$ART/session.log" &&
      verdict INCONCLUSIVE "the session never reached klin, and the host reported an authentication or quota problem"
    verdict COMPAT "the host ran a session but invoked no klin lifecycle hook"
  fi

  PLACED=$(jq -r 'select(.kind == "stop") | .host' "$JOURNAL" | tail -1)
  STOPS=$(jq -c 'select(.kind == "stop")' "$JOURNAL" | wc -l)
  if [ "$STOPS" -eq 0 ]; then
    verdict COMPAT "lifecycle hooks ran, but the host invoked no Stop hook"
  fi
  if [ "$PLACED" != "$HOST" ]; then
    verdict COMPAT "klin placed the Stop event as host '$PLACED', not '$HOST'"
  fi

  if [ ! -s "$ART/tree.txt" ]; then
    verdict INCONCLUSIVE "the host ran klin's lifecycle, but the turn never changed README.md, so no gate could fail"
  fi

  BLOCKED=$(jq -c 'select(.kind == "stop") | .hook.blocked' "$JOURNAL" | grep -c true)
  if [ "$BLOCKED" -eq 0 ]; then
    verdict COMPAT "the turn left a failing tree, but no stop blocked"
  fi
  grep -q 'klin:' "$ART/session.log" ||
    verdict COMPAT "the host ignored klin's block: the report reached no session channel"

  verdict PASS "the host loaded the plugin, ran the lifecycle, invoked Stop and honored klin's failing report"
}

command -v jq > /dev/null || verdict INFRA "the runner has no jq, so the canary cannot read klin's files or its evidence"
install_host
repo
install_plugin
session
evidence
