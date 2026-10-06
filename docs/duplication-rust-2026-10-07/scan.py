"""Rust regions that a commit introduces, with the counterpart verified in the parent.

Incremental copy of the uncommitted v3 scanner (/tmp/klin480-g4-blind/introduced_fast.py)
with two changes for #494: only Rust pairs are kept, and a pair is kept only when the
counterpart's exact normalized tokens occur in the same path in the parent commit.

    python3 scan.py OUT REPO=GIT_DIR:ROOT:LIMIT ...
"""
import bisect
import hashlib
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path, PurePosixPath

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "duplication-speed-2026-10-05"))
import introduced  # noqa: E402

K, T, ADDED_SHARE = introduced.K, introduced.T, introduced.ADDED_SHARE


def scan(name, repo, root, limit, out, blobs):
    commits = introduced.git(repo, "rev-list", "--no-merges", f"--max-count={limit}", "HEAD", "--", root).decode().split()
    commits.reverse()
    if not commits:
        return []
    regions, seen = [], set()
    postings = defaultdict(list)
    indexed = {}

    def add_version(path, sha):
        stream = blobs.stream(sha, PurePosixPath(path).suffix)
        entries = []
        for position in stream["positions"]:
            key = tuple(stream["tokens"][position:position + K])
            if len(key) == K and None not in key:
                entry = (path, position, sha)
                rows = postings[key]
                if rows and rows[-1] > entry:
                    bisect.insort(rows, entry)
                else:
                    rows.append(entry)
                entries.append((key, entry))
        indexed[path, sha] = entries

    def remove_version(path, sha):
        for key, entry in indexed.pop((path, sha), []):
            rows = postings[key]
            index = bisect.bisect_left(rows, entry)
            assert rows[index] == entry
            rows.pop(index)
            if not rows:
                del postings[key]

    unverified = [0]

    def in_parent(path, content):
        sha = files.get(path)
        if sha is None:
            return False
        tokens = blobs.stream(sha, PurePosixPath(path).suffix)["tokens"]
        return f"\n{content}\n" in "\n" + "\n".join(t if t is not None else "\0" for t in tokens) + "\n"

    oldest_parent = introduced.git(repo, "rev-parse", f"{commits[0]}^").decode().strip()
    files = introduced.tree(repo, oldest_parent, root)
    for path in sorted(files):
        add_version(path, files[path])

    for commit in commits:
        try:
            added = introduced.added_lines(repo, commit, root)
        except subprocess.CalledProcessError:
            added = {}
        new_files = introduced.tree(repo, commit, root)
        for path in sorted(set(files) | set(new_files)):
            old_sha, new_sha = files.get(path), new_files.get(path)
            if old_sha != new_sha:
                if old_sha is not None:
                    remove_version(path, old_sha)
                if new_sha is not None:
                    add_version(path, new_sha)
        changed = {}
        for path, lines in added.items():
            ok, relative = introduced.eligible(root, path)
            if ok and relative in new_files:
                changed[relative] = lines
        if changed:
            for path, lines in changed.items():
                stream = blobs.stream(new_files[path], PurePosixPath(path).suffix)
                a, rows = stream["tokens"], stream["rows"]
                coverage = {}
                for position in stream["positions"]:
                    if rows[position] + 1 not in lines:
                        continue
                    key = tuple(a[position:position + K])
                    if len(key) != K or None in key:
                        continue
                    for other, start, other_sha in postings.get(key, ()):
                        if (other, start) == (path, position) and other_sha == new_files[path]:
                            continue
                        diagonal = start - position
                        prior = coverage.get((other, diagonal))
                        if prior and prior[0] <= position < prior[1]:
                            continue
                        other_stream = blobs.stream(other_sha, PurePosixPath(other).suffix)
                        b = other_stream["tokens"]
                        left, right = position, position + K
                        while left > 0 and left + diagonal > 0 and a[left-1] is not None and a[left-1] == b[left+diagonal-1]:
                            left -= 1
                        while right < len(a) and right + diagonal < len(b) and a[right] is not None and a[right] == b[right+diagonal]:
                            right += 1
                        coverage[other, diagonal] = (left, right)
                        if right - left < T or not path.endswith(".rs"):
                            continue
                        new_rows = {rows[i] + 1 for i in range(left, right)}
                        if len(new_rows & lines) < ADDED_SHARE * len(new_rows):
                            continue
                        content = "\n".join(a[left:right])
                        if not in_parent(other, content):
                            unverified[0] += 1
                            continue
                        family = hashlib.sha256(content.encode()).hexdigest()
                        other_rows = other_stream["rows"]
                        sides = [(path, rows, left), (other, other_rows, left + diagonal)]
                        spans = []
                        for p, row_numbers, offset in sides:
                            spans.append({
                                "repo": f"{name}@{commit[:10]}",
                                "path": p,
                                "language": "Rust" if p.endswith(".rs") else "TypeScript",
                                "start_line": row_numbers[offset] + 1,
                                "end_line": row_numbers[offset + right - left - 1] + 1,
                            })
                        identity = (family, spans[0]["path"], spans[1]["path"])
                        if identity in seen:
                            continue
                        seen.add(identity)
                        for span in spans:
                            target = out / "corpus" / span["repo"] / span["path"]
                            if not target.exists():
                                target.parent.mkdir(parents=True, exist_ok=True)
                                target.write_text(blobs.text(new_files[span["path"]]))
                            text = target.read_text().splitlines()
                            span["context"] = "\n".join(text[max(0, span["start_line"] - 4):span["end_line"] + 3])
                        pair_id = hashlib.sha256(json.dumps(
                            [{k: v for k, v in span.items() if k != "context"} for span in spans],
                            sort_keys=True,
                        ).encode()).hexdigest()[:16]
                        regions.append({
                            "id": pair_id,
                            "tokens": right - left,
                            "language": spans[0]["language"],
                            "family": family,
                            "commit": commit,
                            "spans": spans,
                        })
        files = new_files
        print(name, commit[:10], len(regions), flush=True)
    print(name, "unverified counterparts", unverified[0], flush=True)
    scan.unverified[name] = unverified[0]
    return regions


scan.unverified = {}


def main():
    out = Path(sys.argv[1])
    regions, counts = [], {}
    for spec in sys.argv[2:]:
        name, rest = spec.split("=", 1)
        repo, root, limit = rest.rsplit(":", 2)
        found = scan(name, repo, root, int(limit), out, introduced.Blobs(repo, out / "blobs" / name))
        counts[name] = {"commit_limit": int(limit), "root": root, "pairs": len(found), "unverified_dropped": scan.unverified[name]}
        regions += found
    regions.sort(key=lambda row: row["id"])
    blind = [{"id": row["id"], "spans": [dict(span) for span in row["spans"]]} for row in regions]
    for row in regions:
        for span in row["spans"]:
            span.pop("context")
    (out / "mapping.json").write_text(json.dumps({"counts": counts, "matches": regions}, indent=2) + "\n")
    (out / "blind.jsonl").write_text("".join(json.dumps(row) + "\n" for row in blind))
    print(counts)


if __name__ == "__main__":
    main()

