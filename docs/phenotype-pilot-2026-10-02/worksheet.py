"""Merge the detector output into one worksheet that names no arm.

usage: python3 worksheet.py
Writes worksheet.tsv (one row per finding, in a hash order) and holes.tsv.
"""

import collections
import hashlib
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
LABELED = {"new", "worsened"}


def rows():
    for run in sorted((HERE / "runs").glob("*.json")):
        record = json.loads(run.read_text())
        for finding in (record.get("report") or {}).get("findings") or []:
            yield {
                "change": run.stem,
                "family": f"klin:{finding.get('gate')}",
                "file": finding.get("file") or "-",
                "line": finding.get("line") or 0,
                "text": (finding.get("text") or "").strip(),
                "detail": finding.get("outcome") or "-",
                "values": json.dumps(finding.get("values") or {}, sort_keys=True),
            }
    for line in (HERE / "shapes.tsv").read_text().splitlines():
        change, candidate, shape, file, number, tags, text = line.split("\t", 6)
        yield {"change": change, "family": f"shapes:{candidate}", "file": file, "line": int(number),
               "text": text, "detail": tags if tags != "-" else "finding", "values": shape}
    seen = set()
    for line in (HERE / "deps.tsv").read_text().splitlines():
        change, language, variant, identity, where, status, tag = line.split("\t")
        if (change, identity, status) in seen:
            continue
        seen.add((change, identity, status))
        file, _, number = where.rpartition(":")
        yield {"change": change, "family": "deps:undeclared", "file": file, "line": int(number or 0),
               "text": identity, "detail": "finding" if status == "finding" else tag, "values": language}
    for line in (HERE / "tests.tsv").read_text().splitlines():
        change, family, file, number, text = line.split("\t", 4)
        yield {"change": change, "family": f"tests:{family}", "file": file, "line": int(number),
               "text": text, "detail": "finding", "values": "-"}


def main():
    findings, holes = [], collections.Counter()
    for row in rows():
        if row["family"].startswith("klin:") and row["detail"] not in LABELED:
            holes[(row["change"], row["family"], row["detail"])] += 1
        elif row["detail"] in ("finding",) or (row["family"].startswith("klin:") and row["detail"] in LABELED):
            findings.append(row)
        else:
            holes[(row["change"], row["family"], f"excluded {row['detail']}")] += 1
    findings.sort(key=lambda row: hashlib.sha256(json.dumps(row, sort_keys=True).encode()).hexdigest())
    columns = ["id", "change", "family", "file", "line", "detail", "values", "text"]
    with open(HERE / "worksheet.tsv", "w") as out:
        out.write("\t".join(columns) + "\n")
        for number, row in enumerate(findings, 1):
            row["id"] = f"w{number:03d}"
            out.write("\t".join(str(row[column]).replace("\t", " ").replace("\n", " ") for column in columns) + "\n")
    with open(HERE / "holes.tsv", "w") as out:
        out.write("change\tfamily\tkind\tcount\n")
        for (change, family, kind), count in sorted(holes.items()):
            out.write(f"{change}\t{family}\t{kind}\t{count}\n")
    print(f"{len(findings)} findings, {sum(holes.values())} holes or excluded sites")


if __name__ == "__main__":
    main()
