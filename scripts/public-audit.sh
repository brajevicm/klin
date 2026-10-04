#!/usr/bin/env bash
set -euo pipefail

die() {
  echo "public-audit: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

for tool in git gh jq gitleaks trufflehog; do
  need "$tool"
done

root=$(git rev-parse --show-toplevel 2>/dev/null) || die "run inside a git checkout"
cd "$root"

if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  die "working tree is not clean; run from the exact clean commit intended to become public"
fi

gh auth status >/dev/null 2>&1 || die "gh is not authenticated"
repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
[[ -n "$repo" ]] || die "could not determine GitHub repository"

stamp=${PUBLIC_AUDIT_STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}
out=${PUBLIC_AUDIT_OUT:-"$root/target/public-audit/$stamp"}
mkdir -p "$out"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/klin-public-audit.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
touch "$tmp/empty-gitleaksignore"

sanitize_gitleaks() {
  local raw=$1
  local dest=$2
  jq 'map({
    RuleID,
    Description,
    File,
    StartLine,
    EndLine,
    Commit,
    Author,
    Email,
    Date,
    Message,
    Fingerprint
  })' "$raw" > "$dest"
}

run_gitleaks_git() {
  local dest=$1
  local raw="$tmp/gitleaks-$(basename "$dest").raw.json"
  gitleaks git "$root" \
    --no-banner \
    --redact=100 \
    --ignore-gitleaks-allow \
    --gitleaks-ignore-path "$tmp/empty-gitleaksignore" \
    --log-opts="--full-history --all" \
    --report-format json \
    --report-path "$raw" \
    --exit-code 0
  sanitize_gitleaks "$raw" "$dest"
}

run_gitleaks_dir() {
  local source=$1
  local dest=$2
  local raw="$tmp/gitleaks-$(basename "$dest").raw.json"
  gitleaks dir "$source" \
    --no-banner \
    --redact=100 \
    --ignore-gitleaks-allow \
    --gitleaks-ignore-path "$tmp/empty-gitleaksignore" \
    --max-archive-depth 5 \
    --report-format json \
    --report-path "$raw" \
    --exit-code 0
  sanitize_gitleaks "$raw" "$dest"
}

sanitize_trufflehog() {
  jq -c '{
    DetectorName,
    DetectorType,
    DecoderName,
    Verified,
    VerificationError,
    SourceMetadata
  }'
}

run_trufflehog_git() {
  local dest=$1
  trufflehog git "file://$root" \
    --results=verified,unknown,unverified \
    --no-ignore-tag \
    --json \
    | sanitize_trufflehog > "$dest"
}

run_trufflehog_fs() {
  local source=$1
  local dest=$2
  trufflehog filesystem "$source" \
    --results=verified,unknown,unverified \
    --no-ignore-tag \
    --archive-max-depth=5 \
    --archive-max-size=512MB \
    --json \
    | sanitize_trufflehog > "$dest"
}

echo "==> Fetching refs"
git fetch --all --tags --prune

head_sha=$(git rev-parse HEAD)
printf '%s\n' "$head_sha" > "$out/head.txt"
{
  git --version
  gh --version | head -n 1
  jq --version
  gitleaks version
  trufflehog --version
} > "$out/tool-versions.txt"

git for-each-ref \
  --format='%(refname)%09%(objectname)' \
  refs/heads refs/remotes refs/tags \
  > "$out/refs.tsv"

git log --all \
  --format='%H%x09%aI%x09%an%x09%ae' \
  > "$out/commit-authors.tsv"
git log --all --format='%an <%ae>' | sort -u > "$out/commit-identities.txt"

privacy_re='/Users/|/home/|BEGIN [A-Z0-9 ]*PRIVATE KEY|([Aa][Pp][Ii][_-]?[Kk][Ee][Yy]|[Pp][Aa][Ss][Ss][Ww][Oo][Rr][Dd]|[Tt][Oo][Kk][Ee][Nn]|[Ss][Ee][Cc][Rr][Ee][Tt])[[:space:]]*[:=]'
if [[ -n "$(git rev-list --all --max-count=1)" ]]; then
  git rev-list --all \
    | xargs -n 100 git grep -Il -E "$privacy_re" 2>/dev/null \
    | sed -E 's/^[0-9a-f]{40}://' \
    | sort -u \
    > "$out/privacy-history-files.txt" || true
else
  : > "$out/privacy-history-files.txt"
fi

echo "==> Scanning all reachable Git history"
run_gitleaks_git "$out/gitleaks-history.json"
run_trufflehog_git "$out/trufflehog-history.jsonl"

release_root="$tmp/releases"
mkdir -p "$release_root"
: > "$tmp/release-metadata.jsonl"
gh release list --repo "$repo" --limit 1000 --json tagName --jq '.[].tagName' \
  > "$out/release-tags.txt"

echo "==> Downloading release assets"
while IFS= read -r tag; do
  [[ -n "$tag" ]] || continue
  safe_tag=${tag//\//__}
  mkdir -p "$release_root/$safe_tag"
  gh release download "$tag" --repo "$repo" --dir "$release_root/$safe_tag"
  gh release view "$tag" --repo "$repo" \
    --json tagName,name,isDraft,isPrerelease,publishedAt,body,url \
    >> "$tmp/release-metadata.jsonl"
done < "$out/release-tags.txt"
cp "$tmp/release-metadata.jsonl" "$release_root/release-metadata.jsonl"

echo "==> Scanning release assets and release descriptions"
run_gitleaks_dir "$release_root" "$out/gitleaks-releases.json"
run_trufflehog_fs "$release_root" "$out/trufflehog-releases.jsonl"

echo "==> Scanning GitHub issue and pull-request comments"
github_token=$(gh auth token)
env GITHUB_TOKEN="$github_token" \
  trufflehog github \
    --repo="https://github.com/$repo" \
    --issue-comments \
    --pr-comments \
    --results=verified,unknown,unverified \
    --no-ignore-tag \
    --json \
  | sanitize_trufflehog > "$out/trufflehog-github-comments.jsonl"
unset github_token

actions_root="$tmp/actions"
logs_root="$actions_root/logs"
artifacts_root="$actions_root/artifacts"
mkdir -p "$logs_root" "$artifacts_root"
: > "$out/actions-unavailable-logs.txt"

echo "==> Downloading currently accessible Actions logs"
gh api --paginate "repos/$repo/actions/runs?per_page=100" \
  --jq '.workflow_runs[] | [.id, .run_attempt] | @tsv' \
  > "$tmp/action-run-attempts.tsv"

while IFS=$'\t' read -r run_id run_attempt; do
  [[ -n "$run_id" ]] || continue
  attempt=1
  while ((attempt <= run_attempt)); do
    dest="$logs_root/$run_id-$attempt.zip"
    if ! gh api "repos/$repo/actions/runs/$run_id/attempts/$attempt/logs" > "$dest" 2>/dev/null; then
      rm -f "$dest"
      printf '%s\t%s\n' "$run_id" "$attempt" >> "$out/actions-unavailable-logs.txt"
    fi
    attempt=$((attempt + 1))
  done
done < "$tmp/action-run-attempts.tsv"

echo "==> Downloading non-expired Actions artifacts"
gh api --paginate "repos/$repo/actions/artifacts?per_page=100" \
  --jq '.artifacts[] | select(.expired == false) | [.id, .workflow_run.id, .name] | @tsv' \
  > "$out/actions-artifacts.tsv"

while IFS=$'\t' read -r artifact_id run_id artifact_name; do
  [[ -n "$artifact_id" ]] || continue
  gh api "repos/$repo/actions/artifacts/$artifact_id/zip" \
    > "$artifacts_root/${artifact_id}.zip"
done < "$out/actions-artifacts.tsv"

echo "==> Scanning Actions logs and artifacts"
run_gitleaks_dir "$actions_root" "$out/gitleaks-actions.json"
run_trufflehog_fs "$actions_root" "$out/trufflehog-actions.jsonl"

gitleaks_history=$(jq 'length' "$out/gitleaks-history.json")
gitleaks_releases=$(jq 'length' "$out/gitleaks-releases.json")
gitleaks_actions=$(jq 'length' "$out/gitleaks-actions.json")
truffle_history=$(wc -l < "$out/trufflehog-history.jsonl" | tr -d ' ')
truffle_releases=$(wc -l < "$out/trufflehog-releases.jsonl" | tr -d ' ')
truffle_comments=$(wc -l < "$out/trufflehog-github-comments.jsonl" | tr -d ' ')
truffle_actions=$(wc -l < "$out/trufflehog-actions.jsonl" | tr -d ' ')
privacy_files=$(wc -l < "$out/privacy-history-files.txt" | tr -d ' ')
unavailable_logs=$(wc -l < "$out/actions-unavailable-logs.txt" | tr -d ' ')

cat > "$out/SUMMARY.txt" <<EOF
klin public-visibility audit
candidate: $head_sha
repository: $repo

Gitleaks findings
  Git history:      $gitleaks_history
  release material: $gitleaks_releases
  Actions material: $gitleaks_actions

TruffleHog findings
  Git history:      $truffle_history
  release material: $truffle_releases
  GitHub comments:  $truffle_comments
  Actions material: $truffle_actions

Manual review
  privacy-marker files in reachable history: $privacy_files
  Actions run attempts whose logs were unavailable: $unavailable_logs

Reports: $out

A completed scan is not an automatic pass. Classify every scanner finding.
The publication gate is: zero unexplained or real credentials, plus explicit
human approval of commit identities, privacy-marker locations, and unavailable
Actions logs before changing repository visibility.
EOF

cat "$out/SUMMARY.txt"
