#!/usr/bin/env bash
# scope.sh: what each tool's report says about the files it read, over a tree
# with a clean file, a file that does not parse, a file outside the
# TypeScript project, and an ignored file.
set -u
here=$(cd "$(dirname "$0")" && pwd)
tools=${TOOLS:?TOOLS names the directory of the pinned tools}
w=$(mktemp -d)
trap 'rm -rf "$w"' EXIT
mkdir -p "$w/src" "$w/ignored"
cd "$w" || exit 1
git init -q
printf 'ignored/\n.klin-recipes/\n' > .gitignore
printf '{ "compilerOptions": { "strict": true, "target": "ES2022", "module": "ESNext", "moduleResolution": "Bundler", "lib": ["ES2022", "DOM"], "noEmit": true }, "include": ["src"] }\n' > tsconfig.json
printf 'export const a = 1;\n' > src/clean.ts
printf 'export const b = (;\n' > src/broken.ts
printf 'export const c = 1;\n' > outside.ts
printf 'console.log(1);\nexport {};\n' > ignored/hidden.ts
printf 'x = 1\n' > src/clean.py
printf 'def f(:\n' > src/broken.py
printf 'print(1)\n' > ignored/hidden.py
for tool in eslint ruff semgrep gitleaks; do
  sh "$here/recipes/run.sh" "$tool" "$tools" "$here/recipes" > /dev/null 2>&1
  echo "$tool: $(jq -c '{artifacts: [.runs[].artifacts[]?.location.uri | sub(".*/"; "")],
    results: [.runs[].results[] | "\(.ruleId) \(.locations[0].physicalLocation.artifactLocation.uri | sub(".*/"; ""))"],
    invocations: [.runs[].invocations[]? | {ok: .executionSuccessful,
      notes: [(.toolExecutionNotifications[]?, .toolConfigurationNotifications[]?) | .message.text[0:60]]}]}' ".klin-recipes/$tool.sarif")"
done
echo "semgrep json: $("$tools/py/.venv/bin/semgrep" scan --config "$here/recipes/semgrep.yml" --metrics=off --disable-version-check --quiet --json . | jq -c '{scanned: .paths.scanned, errors: [.errors[] | .path]}')"
