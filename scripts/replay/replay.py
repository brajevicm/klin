#!/usr/bin/env python3
"""Precision replay of one klin row over the history of real apps (#572).

Local only: it clones apps and runs klin once per change, so CI never runs it.

  replay.py sample SET REPO_URL... [--changes 100]
      Freeze SET: the newest non-merge commits of each repository, at least
      --changes in all. A frozen sample is never rewritten.
  replay.py run SET ROW [--klin PATH]
      Run klin over every change of SET, keep the findings of ROW, append each
      finding the labels file lacks as "?" for an agent to draft, and print
      N, the not-appropriate findings per 100 changes.

ROW is CHECK, CHECK:KEY or CHECK:KEY=VALUE, the catch rule of tests/recall.rs:
a finding of CHECK whose values[KEY] exists or equals VALUE. A ride-along
review item counts as a finding.

Labels live in labels/ROW--SET.tsv, one finding per line:
commit, file, line, label (appropriate, not-appropriate or ?), reason, and
reviewed (the person who checked it, or empty). Before a row may block, a
person reviews every finding up to 30, and a fixed random 30 above that.
"""

import argparse
import hashlib
import json
import math
import os
import random
import re
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
WORK = ROOT / "target" / "replay"
SPOT = 30
COLUMNS = ["commit", "file", "line", "label", "reason", "reviewed"]


def git(cwd, *args):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


def slug(text):
    return re.sub(r"[^A-Za-z0-9]+", "-", text).strip("-").lower()


def clone(url):
    into = WORK / "clones" / slug(url)
    if not into.exists():
        subprocess.run(["git", "clone", "--quiet", "--no-tags", url, str(into)], check=True)
    return into


def checkout(url):
    into = WORK / "trees" / slug(url)
    if not into.exists():
        subprocess.run(["git", "clone", "--quiet", str(clone(url)), str(into)], check=True)
        git(into, "remote", "remove", "origin")
    return into


def sample(args):
    path = HERE / "samples" / f"{args.set}.json"
    if path.exists():
        sys.exit(f"{path} is frozen; a new sample needs a new set name")
    per = math.ceil(args.changes / len(args.repos))
    changes = []
    for url in args.repos:
        heads = git(clone(url), "rev-list", "--no-merges", f"--max-count={per}", "HEAD").split()
        changes += [{"repo": url, "commit": head} for head in heads]
    path.parent.mkdir(exist_ok=True)
    path.write_text(json.dumps({"changes": changes}, indent=2) + "\n")
    print(f"froze {len(changes)} changes of {len(args.repos)} repositories in {path}")


def prepare(cwd, commit):
    git(cwd, "checkout", "--quiet", "--force", "--detach", commit)
    for branch in git(cwd, "for-each-ref", "--format=%(refname:short)", "refs/heads").split():
        git(cwd, "branch", "--quiet", "-D", branch)
    git(cwd, "branch", "--quiet", "main", f"{commit}^")
    git(cwd, "checkout", "--quiet", "-b", "change")
    git(cwd, "clean", "--quiet", "-ffdx")
    shutil.rmtree(cwd / ".git" / "klin", ignore_errors=True)
    (cwd / "klin.json").write_text("{}\n")


def report(klin, version, change, check):
    cached = WORK / "runs" / version / check / f"{change['commit']}.json"
    if cached.exists():
        return json.loads(cached.read_text())
    cwd = checkout(change["repo"])
    try:
        prepare(cwd, change["commit"])
    except subprocess.CalledProcessError as why:
        return {"failed": str(why)}
    env = {name: value for name, value in os.environ.items() if not name.startswith(("KLIN_", "GITHUB_"))}
    done = subprocess.run([klin, "check", check, "--json"], cwd=cwd, env=env, capture_output=True, text=True)
    try:
        result = json.loads(done.stdout)
    except json.JSONDecodeError:
        result = {"failed": done.stdout + done.stderr}
    cached.parent.mkdir(parents=True, exist_ok=True)
    cached.write_text(json.dumps(result))
    return result


def matches(rule, finding):
    check, _, kind = rule.partition(":")
    key, _, value = kind.partition("=")
    found = (finding.get("values") or {}).get(key)
    wanted = not key or (found is not None and (not value or str(found) == value))
    return finding.get("check") == check and wanted


def read_labels(path):
    if not path.exists():
        return {}
    rows = [line.split("\t") for line in path.read_text().splitlines()[1:] if line]
    return {tuple(row[:3]): dict(zip(COLUMNS, row + [""] * len(COLUMNS))) for row in rows}


def write_labels(path, labels):
    path.parent.mkdir(exist_ok=True)
    lines = ["\t".join(COLUMNS)] + ["\t".join(label[column] for column in COLUMNS) for label in labels.values()]
    path.write_text("\n".join(lines) + "\n")


def run(args):
    changes = json.loads((HERE / "samples" / f"{args.set}.json").read_text())["changes"]
    klin = str(Path(args.klin).resolve())
    version = hashlib.sha256(Path(klin).read_bytes()).hexdigest()[:16]
    check = args.row.partition(":")[0]
    path = HERE / "labels" / f"{slug(args.row)}--{args.set}.tsv"
    labels = read_labels(path)
    failed = []
    found = []
    for change in changes:
        result = report(klin, version, change, check)
        if result.get("measurement") != "complete":
            failed.append(change["commit"])
        riders = [item for item in result.get("reviews", []) if item.get("kind") == "ride-along"]
        for finding in result.get("findings", []) + riders:
            if matches(args.row, finding):
                found.append((change["commit"], finding["file"], str(finding["line"])))
    for key in found:
        labels.setdefault(key, dict(zip(COLUMNS, [*key, "?", "", ""])))
    write_labels(path, labels)
    summarize(f"{args.row}--{args.set}", changes, failed, [labels[key] for key in found])


def summarize(name, changes, failed, findings):
    measured = len(changes) - len(failed)
    wrong = sum(label["label"] == "not-appropriate" for label in findings)
    unlabeled = sum(label["label"] not in ("appropriate", "not-appropriate") for label in findings)
    spot = findings if len(findings) <= SPOT else random.Random(name).sample(findings, SPOT)
    unreviewed = [label for label in spot if not label["reviewed"]]
    print(f"{name}: {measured} changes measured, {len(failed)} not measured in full, {len(findings)} findings")
    print(f"N = {100 * wrong / max(measured, 1):.1f} not-appropriate findings per 100 changes")
    if unlabeled:
        print(f"not final: {unlabeled} finding(s) carry no label")
    print(f"spot-check: {len(spot) - len(unreviewed)} of {len(spot)} reviewed by a person")
    for label in unreviewed:
        print(f"  {label['commit'][:12]} {label['file']}:{label['line']} {label['label']} {label['reason']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    freeze = commands.add_parser("sample")
    freeze.set_defaults(command=sample)
    freeze.add_argument("set")
    freeze.add_argument("repos", nargs="+")
    freeze.add_argument("--changes", type=int, default=100)
    replay = commands.add_parser("run")
    replay.set_defaults(command=run)
    replay.add_argument("set")
    replay.add_argument("row")
    replay.add_argument("--klin", default=str(ROOT / "target" / "release" / "klin"))
    args = parser.parse_args()
    args.command(args)


if __name__ == "__main__":
    main()
