#!/usr/bin/env python3
"""Census and history replay for #425.

census: every function of HEAD as a complexity site, with the prototype's identity state.
replay: for the last N commits that are no merge, the commit against its parent, under the
`--changed` run, from each binary. A difference is a site whose outcome the two binaries
disagree on.

usage: replay.py LABEL REPO N BASE_BINARY PROTO_BINARY OUT_DIR
N=0 runs the census alone.
"""

import collections
import os
import json
import subprocess
import sys
import tempfile
from pathlib import Path

CONFIG = os.environ.get("REPLAY_CONFIG", '{ "complexity": { "cc": 0, "lines": 1000000 } }')


def git(root, *args):
    return subprocess.run(
        ["git", "-c", "user.name=probe", "-c", "user.email=probe@example.com",
         "-c", "commit.gpgsign=false", *args],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()


def at(root, commit, base):
    git(root, "checkout", "-q", "-f", "-B", "work", commit)
    git(root, "clean", "-q", "-fdx", "-e", ".git")
    git(root, "branch", "-f", "main", base)
    (root / "klin.json").write_text(CONFIG)
    subprocess.run(["rm", "-rf", str(root / ".git" / "klin")], check=True)


def findings(binary, root, changed, every=False):
    args = [binary, "gate", "--gate", "complexity", "--json"]
    if changed:
        args.append("--changed")
    run = subprocess.run(args, cwd=root, capture_output=True, text=True)
    data = json.loads(run.stdout)
    sites = {}
    for at, finding in enumerate(data["findings"]):
        if finding.get("outcome") not in ("new", "worsened") or "file" not in finding:
            continue
        key = (finding["file"], finding.get("line", 0), finding["text"])
        if every:
            key += (at,)
        sites[key] = (finding["outcome"], finding["values"])
    return sites, data["gates"][0].get("held", 0), data["exit"]


def census(label, root, proto, out):
    empty = git(root, "commit-tree", git(root, "hash-object", "-t", "tree", "/dev/null"), "-m", "empty")
    head = git(root, "commit-tree", "HEAD^{tree}", "-p", empty, "-m", "head")
    at(root, head, empty)
    sites, _, _ = findings(proto, root, False, every=True)
    states = collections.Counter()
    over_floor = collections.Counter()
    by_text = collections.Counter((file, text) for file, _, text, _ in sites)
    shared_text = sum(count for count in by_text.values() if count > 1)
    shared_identified = 0
    for (file, _, text, _), (_, values) in sites.items():
        identity = values.get("identity", "")
        state = "identified" if ":" in identity else identity.split("?", 1)[-1]
        states[state] += 1
        if values.get("cc", 0) > 10:
            over_floor[state] += 1
        if by_text[(file, text)] > 1 and state == "identified":
            shared_identified += 1
    result = {
        "label": label,
        "functions": len(sites),
        "states": dict(states),
        "states_over_cc_10": dict(over_floor),
        "sites_sharing_file_and_text": shared_text,
        "of_those_identified": shared_identified,
    }
    (out / f"{label}-census.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def replay(label, root, tip, count, base, proto, out):
    commits = git(root, "rev-list", "--no-merges", f"--max-count={count}", tip).split()
    rows = []
    totals = collections.Counter()
    for commit in commits:
        parent = git(root, "rev-parse", f"{commit}^")
        at(root, commit, parent)
        before, held_before, exit_before = findings(base, root, True)
        after, held_after, exit_after = findings(proto, root, True)
        if exit_before != exit_after:
            totals[f"verdict exit {exit_before}->{exit_after}"] += 1
        totals["commits"] += 1
        totals["base_findings"] += len(before)
        totals["proto_findings"] += len(after)
        totals["base_held"] += held_before
        totals["proto_held"] += held_after
        for key in sorted(set(before) | set(after)):
            was = before.get(key, ("held", {}))[0]
            now = after.get(key, ("held", {}))[0]
            if was == now:
                continue
            identity = after.get(key, before.get(key))[1].get("identity", "")
            totals[f"{was}->{now}"] += 1
            rows.append([commit[:10], *map(str, key), was, now, identity])
    (out / f"{label}-replay.tsv").write_text(
        "commit\tfile\tline\ttext\tbase\tproto\tidentity\n"
        + "".join("\t".join(row) + "\n" for row in rows)
    )
    result = {"label": label, **totals}
    (out / f"{label}-replay.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    label, repo, count, base, proto, out = sys.argv[1:]
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix=f"replay-{label}-"))
    subprocess.run(["git", "clone", "-q", "--local", repo, str(root)], check=True)
    git(root, "remote", "remove", "origin")
    tip = git(root, "rev-parse", "HEAD")
    print(json.dumps(census(label, root, proto, out)))
    if int(count):
        print(json.dumps(replay(label, root, tip, int(count), base, proto, out)))
    subprocess.run(["rm", "-rf", str(root)], check=True)


if __name__ == "__main__":
    main()
