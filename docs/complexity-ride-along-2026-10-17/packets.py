#!/usr/bin/env python3
"""Write one labeling packet per finding that a candidate keeps.

  packets.py SET OUT [--klin PATH]
  packets.py --check

OUT gets packets/P###.md, label-rule.md and repos/ with a partial clone of
each repository, which is all a labeler sees. key.tsv, the map from packet
to finding with the candidates that keep it, goes beside OUT, never in it.
A packet shows the pull request title, the function at the head, the
change to it and its "before". It shows no klin number and no rule.
"""

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

import rules

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
LONGEST = 400
EXCEPTION = """For this labeling (#697), a change is one pull request, from its merge base
to its head, not a commit against its first parent. The packet names both
commits. Use them, not the parent commit, when you check with git.

"""


def git(cwd, *args):
    return subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True).stdout


def repository(out, change):
    name = change["repo"].removeprefix("https://github.com/")
    into = out / "repos" / name.replace("/", "__")
    if not into.exists():
        subprocess.run(["git", "clone", "--quiet", "--no-tags", "--no-checkout", "--filter=blob:none", change["repo"], str(into)], check=True)
    git(into, "fetch", "--quiet", "origin", f"+pull/{change['pull']}/head:refs/pull/{change['pull']}")
    return name, into


def title(name, number):
    done = subprocess.run(["gh", "api", f"repos/{name}/pulls/{number}", "-q", ".title"], capture_output=True, text=True)
    return done.stdout.strip() or "(title not readable)"


QUOTED = {
    "rust": re.compile(r'"(\\.|[^"\\])*"|\'(\\.|[^\'\\])\'|//.*'),
    "other": re.compile(r'"(\\.|[^"\\])*"|\'(\\.|[^\'\\])*\'|`(\\.|[^`\\])*`|//.*'),
}


def extent(lines, start, length, path):
    """The 1-based last line of the function that starts at line start."""
    first = lines[start - 1]
    if first.lstrip().startswith(("def ", "async def ")):
        indent = len(first) - len(first.lstrip())
        for number in range(start + 1, min(len(lines), start + LONGEST) + 1):
            text = lines[number - 1]
            if text.strip() and len(text) - len(text.lstrip()) <= indent:
                return number - 1
        return min(len(lines), start + LONGEST)
    quoted = QUOTED["rust" if path.endswith(".rs") else "other"]
    depth = 0
    opened = False
    for number in range(start, min(len(lines), start + LONGEST) + 1):
        code = quoted.sub("", lines[number - 1])
        depth += code.count("{") - code.count("}")
        opened = opened or "{" in code
        if opened and depth <= 0:
            return number
    return min(len(lines), start + (length or 40))


def show(cwd, commit, path, start, length):
    text = git(cwd, "show", f"{commit}:{path}")
    if not text:
        return None, None
    lines = text.splitlines()
    if not 0 < start <= len(lines):
        return None, None
    end = min(len(lines), start + length - 1) if length else extent(lines, start, None, path)
    return (start, end), "\n".join(f"{n:>5}  {lines[n - 1]}" for n in range(start, end + 1))


def hunks(cwd, base, head, path, others, span):
    """The changed lines of path within the span, with three lines of context."""
    diff = git(cwd, "diff", "-M", "-U3", base, head, "--", path, *others)
    low, high = span[0] - 3, span[1] + 3
    kept, current, ours, new = [], None, False, 0
    for line in diff.splitlines():
        if line.startswith("+++ "):
            ours = line == f"+++ b/{path}"
            continue
        found = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@", line)
        if found:
            new = int(found.group(1))
            current = [] if ours else None
            if current is not None:
                kept.append(current)
            continue
        if current is None or line.startswith(("\\", "--- ")):
            continue
        if (span[0] <= new <= span[1]) if line.startswith("-") else (low <= new <= high):
            current.append((new, line))
        if not line.startswith("-"):
            new += 1
    shown = [hunk for hunk in kept if any(line[:1] in "+-" for _, line in hunk)]
    return "\n".join(f"@@ head lines {hunk[0][0]}-{hunk[-1][0]} @@\n" + "\n".join(line for _, line in hunk) for hunk in shown)


def renamed_from(cwd, base, head, path):
    for line in git(cwd, "diff", "-M", "--name-status", base, head).splitlines():
        parts = line.split("\t")
        if parts[0].startswith("R") and parts[-1] == path:
            return parts[1]
    return None


DEFINITION = re.compile(
    r"""^\s*(?:(?:pub(?:\([^)]*\))?|export|default|async|static|public|private|protected|override|unsafe|const|extern(?:\s+"[^"]*")?)\s+)*
    (?:fn\s+(\w+)|function\*?\s+(\w+)|def\s+(\w+)
    |(?:const|let|var)\s+(\w+)\s*(?::[^=]+)?=\s*(?:async\s+)?(?:function\b|\(|\w+\s*=>)
    |(\w+)\s*(?:<[^>]*>)?\s*\([^;]*$)""",
    re.X,
)
NOT_NAMES = {"if", "for", "while", "switch", "catch", "return", "match", "else", "it", "test", "describe", "expect"}


def name_of(text):
    """The name a line defines, or None when it defines no function."""
    found = DEFINITION.match(text)
    name = next((group for group in found.groups() if group), None) if found else None
    return None if name in NOT_NAMES else name


def definitions(cwd, commit, name):
    """Every (file, line) at commit whose line defines a function called name."""
    found = []
    for line in git(cwd, "grep", "-I", "-n", "-w", "-e", name, commit).splitlines():
        parts = line.removeprefix(f"{commit}:").split(":", 2)
        if len(parts) == 3 and name_of(parts[2]) == name:
            found.append((parts[0], int(parts[1])))
    return found


def earlier(cwd, base, name, path, old):
    """Up to three base functions of the same name, the same file first."""
    return sorted(definitions(cwd, base, name), key=lambda item: (item[0] not in (path, old), item))[:3]


def removed(cwd, change, path, old, name):
    """Up to three functions of the file at the base that the change deleted everywhere."""
    gone = []
    for line in git(cwd, "diff", "-M", "-U0", change["base"], change["commit"], "--", path, *([old] if old else [])).splitlines():
        lost = name_of(line[1:]) if line.startswith("-") and not line.startswith("---") else None
        if lost and lost != name and lost not in gone and not definitions(cwd, change["commit"], lost):
            gone.append(lost)
    found = [item for lost in gone for item in definitions(cwd, change["base"], lost) if item[0] in (path, old)]
    return found[:3]


def shown(cwd, commit, places):
    parts = []
    for path, line in places:
        _, code = show(cwd, commit, path, line, None)
        parts.append(f"`{path}` line {line}:\n\n```\n{code or '(not readable)'}\n```")
    return parts


def before(cwd, change, finding, old):
    base = change["base"]
    matched = finding.get("matched")
    name = name_of(finding["text"])
    if matched:
        length = rules_length(matched.get("values") or {})
        _, code = show(cwd, base, matched["file"], matched["line"], length)
        parts = [f"The function existed before this pull request. At the base it is in `{matched['file']}` at line {matched['line']}:\n\n```\n{code or '(not readable)'}\n```"]
    else:
        parts = ["klin found no earlier version of this function, so it counts as new."]
        if old:
            parts.append(f"The pull request renamed the file from `{old}`.")
        found = earlier(cwd, base, name, finding["file"], old) if name else []
        if found:
            parts.append("Functions in the base with the same name, the same file first. Decide with label-rule.md rule 3 whether one is an earlier version of this function:")
            parts += shown(cwd, base, found)
        else:
            parts.append("No function in the base has the same name.")
    gone = removed(cwd, change, finding["file"], old, name)
    if gone:
        parts.append("Functions of this file that the pull request deleted, which may have moved into this function:")
        parts += shown(cwd, base, gone)
    return "\n\n".join(parts) + "\n"


def rules_length(values):
    return values.get("lines", values.get("test_lines"))


def packet(cwd, name, number, pr_title, change, finding):
    old = renamed_from(cwd, change["base"], change["commit"], finding["file"])
    others = {old, (finding.get("matched") or {}).get("file")} - {None, finding["file"]}
    span, code = show(cwd, change["commit"], finding["file"], finding["line"], rules_length(finding["values"]))
    diff = hunks(cwd, change["base"], change["commit"], finding["file"], sorted(others), span) if span else ""
    return f"""Repository: `{name}` (the clone is `repos/{name.replace('/', '__')}`)

Pull request #{number}: {pr_title}

Change: from the merge base `{change['base']}` to the head `{change['commit']}`

Function: `{finding['file']}` line {finding['line']} at the head, `{finding['text'].strip()}`

## The function at the head

```
{code or '(not readable)'}
```

## The change to this function

```diff
{diff or '(no hunk of the diff touches these lines)'}
```

## Before

{before(cwd, change, finding, old)}"""


def check():
    ts = ["export function f({", "  a,", "}: {", "  a: string", "}) {", "  const s = '}{';", "  if (a) { return `${a}}` }", "  // } comment", "}", "after"]
    assert extent(ts, 1, 3, "x.ts") == 9
    rs = ["pub fn g<'a>(", "    x: &'a str,", ") -> Result<Node> {", "    let c = '}';", "    if x.is_empty() {", "        return Err(\"}\");", "    }", "}", "fn h() {}"]
    assert extent(rs, 1, 3, "x.rs") == 8
    py = ["def t():", "    x = 1", "", "    return x", "def u():"]
    assert extent(py, 1, None, "x.py") == 4
    assert name_of("pub(crate) async fn walk<T>(x: T) {") == "walk"
    assert name_of("export const handler = async (req) => {") == "handler"
    assert name_of("  async int_applyHead(newHead: ClockHead) {") == "int_applyHead"
    assert name_of("def test_with_functions():") == "test_with_functions"
    assert name_of("    new_dma.set_state(&state, None)") is None
    assert name_of("    state::{StateComponent, StateFormat},") is None
    assert name_of('it("does x", async () => {') is None
    assert name_of("  if (ready) {") is None
    print("packets.py self-check passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("set", nargs="?")
    parser.add_argument("out", nargs="?")
    parser.add_argument("--klin", default=str(ROOT / "target/release/klin"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check or not args.out:
        return check() if args.check else parser.error("give SET and OUT, or --check")
    out = Path(args.out).resolve()
    (out / "packets").mkdir(parents=True, exist_ok=True)
    rule = (ROOT / "docs/complexity-calibration-2026-10-16/label-rule.md").read_text()
    (out / "label-rule.md").write_text(EXCEPTION + rule)
    changes = json.loads((ROOT / "scripts/replay/samples" / f"{args.set}.json").read_text())["changes"]
    version = hashlib.sha256(Path(args.klin).read_bytes()).hexdigest()[:16]
    runs = ROOT / "target/replay/runs" / version / "complexity"
    kept = []
    for change in changes:
        cached = runs / f"{change['commit']}.json"
        if not cached.exists():
            continue
        result = json.loads(cached.read_text())
        if result.get("measurement") != "complete":
            continue
        for finding in rules.findings(result):
            if rules.keeps(finding):
                kept.append((change, finding))
    kept.sort(key=lambda item: hashlib.sha256(f"697-packet:{item[0]['commit']}:{item[1]['file']}:{item[1]['line']}".encode()).hexdigest())
    rows = ["packet\tcommit\tfile\tline\toutcome\tlanguage\tcandidates"]
    titles = {}
    for number, (change, finding) in enumerate(kept, 1):
        name, cwd = repository(out, change)
        key = (name, change["pull"])
        if key not in titles:
            titles[key] = title(*key)
        packet_id = f"P{number:03d}"
        (out / "packets" / f"{packet_id}.md").write_text(f"# {packet_id}\n\n" + packet(cwd, name, change["pull"], titles[key], change, finding))
        rows.append("\t".join([packet_id, change["commit"], finding["file"], str(finding["line"]), finding["outcome"], change["language"], ",".join(rules.keeps(finding))]))
    (out.parent / f"{out.name}-key.tsv").write_text("\n".join(rows) + "\n")
    print(f"{len(kept)} packets in {out / 'packets'}")


if __name__ == "__main__":
    main()
