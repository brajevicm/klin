#!/usr/bin/env python3
"""The three candidate rows of #697, the definition a shipping ticket ports.

  rules.py SET [--klin PATH]   one line per complexity finding of the changes
                               of SET that klin measured in full, then counts
  rules.py --check             the self-check

It reads the results that scripts/replay/replay.py cached for that binary.
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CANDIDATES = ["first", "second", "cc"]
GROWTH = 20


def ceiling(finding):
    return int(re.search(r"\bcc (\d+)", finding["ceiling"]).group(1))


def keeps(finding):
    values = finding["values"]
    base = (finding.get("matched") or {}).get("values") or {}
    over = values["cc"] > ceiling(finding)
    new = finding["outcome"] == "new"
    worsened = finding["outcome"] == "worsened"
    grew = worsened and any(key in values and key in base and values[key] - base[key] >= GROWTH for key in ("lines", "test_lines"))
    return [
        name
        for name, kept in (
            ("first", (new and over) or grew),
            ("second", over and (new or (worsened and values["cc"] > base.get("cc", values["cc"])))),
            ("cc", over and (new or worsened)),
        )
        if kept
    ]


def cached_at(runs, change):
    return runs / (f"{change['commit']}-{change['base']}.json" if "base" in change else f"{change['commit']}.json")


def findings(result):
    return [item for item in result.get("findings", []) if item.get("check") == "complexity"]


def run(args):
    changes = json.loads((ROOT / "scripts/replay/samples" / f"{args.set}.json").read_text())["changes"]
    version = hashlib.sha256(Path(args.klin).read_bytes()).hexdigest()[:16]
    runs = ROOT / "target/replay/runs" / version / "complexity"
    counts = dict.fromkeys(CANDIDATES, 0)
    measured = 0
    print("commit\tfile\tline\toutcome\tvalues\tbase\tcandidates")
    for change in changes:
        cached = cached_at(runs, change)
        if not cached.exists():
            continue
        result = json.loads(cached.read_text())
        if result.get("measurement") != "complete":
            continue
        measured += 1
        for finding in findings(result):
            kept = keeps(finding)
            for name in kept:
                counts[name] += 1
            base = (finding.get("matched") or {}).get("values") or {}
            print(f"{change['commit']}\t{finding['file']}\t{finding['line']}\t{finding['outcome']}\t{json.dumps(finding['values'])}\t{json.dumps(base)}\t{','.join(kept) or '-'}")
    print(f"{measured} of {len(changes)} changes measured in full; kept: " + ", ".join(f"{name} {counts[name]}" for name in CANDIDATES), file=sys.stderr)


def check():
    def finding(outcome, cc, lines, base=None, ceiling="cc 10, lines 80"):
        matched = {"values": base} if base is not None else None
        return {"outcome": outcome, "values": {"cc": cc, "lines": lines}, "matched": matched, "ceiling": ceiling}

    assert keeps(finding("new", 11, 30)) == ["first", "second", "cc"]
    assert keeps(finding("new", 9, 90)) == []
    assert keeps(finding("worsened", 9, 110, {"cc": 9, "lines": 90})) == ["first"]
    assert keeps(finding("worsened", 9, 109, {"cc": 9, "lines": 90})) == []
    assert keeps(finding("worsened", 12, 85, {"cc": 11, "lines": 84})) == ["second", "cc"]
    assert keeps(finding("worsened", 12, 85, {"cc": 12, "lines": 70})) == ["cc"]
    assert keeps(finding("new", 14, 30, ceiling="cc 15 (the 2026-11-01 step), lines 80")) == []
    tested = {"outcome": "worsened", "values": {"cc": 3, "test_lines": 150}, "matched": {"values": {"cc": 3, "test_lines": 130}}, "ceiling": "cc 10, lines 80, test_lines 120"}
    assert keeps(tested) == ["first"]
    moved = {"outcome": "worsened", "values": {"cc": 3, "test_lines": 150}, "matched": {"values": {"cc": 3, "lines": 100}}, "ceiling": "cc 10, lines 80, test_lines 120"}
    assert keeps(moved) == []
    assert keeps(finding("held", 12, 85, {"cc": 12, "lines": 85})) == []
    print("rules.py self-check passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("set", nargs="?")
    parser.add_argument("--klin", default=str(ROOT / "target/release/klin"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        check()
    elif args.set:
        run(args)
    else:
        parser.error("give SET or --check")


if __name__ == "__main__":
    main()
