#!/usr/bin/env python3
"""Throwaway registry lookup for #364. Not klin code.

registry lookup ECOSYSTEM NAME [COMMIT_DATE]    one lookup, through the cache
registry batch                                  lookups for "ecosystem name date" lines on stdin

The cache is snapshot.tsv beside this file, keyed by ecosystem, name and
snapshot date. A cached row makes no request. OFFLINE=1 makes every uncached
lookup read `unknown`.
"""

import datetime
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, "snapshot.tsv")
SNAPSHOT = os.environ.get("SNAPSHOT", "2026-10-02")
HEADERS = {"User-Agent": "klin-research-364 (https://github.com/brajevicm/klin/issues/364)"}


def load():
    rows = {}
    if os.path.exists(CACHE):
        with open(CACHE) as handle:
            for line in handle:
                ecosystem, name, snapshot, status, first = line.rstrip("\n").split("\t")[:5]
                rows[(ecosystem, name, snapshot)] = (status, first)
    return rows


def fetch(url):
    request = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(request, timeout=10) as response:
        return response.read()


def go_escape(module):
    return "".join("!" + c.lower() if c.isupper() else c for c in module)


def semver_key(version):
    core = version.lstrip("v").split("+")[0]
    main, _, pre = core.partition("-")
    parts = [int(p) if p.isdigit() else 0 for p in main.split(".")]
    return parts, pre == "", pre


def first_release(ecosystem, name):
    if ecosystem == "pypi":
        data = json.loads(fetch(f"https://pypi.org/pypi/{urllib.parse.quote(name)}/json"))
        times = [f["upload_time_iso_8601"] for files in data["releases"].values() for f in files]
        return min(times) if times else ""
    if ecosystem == "npm":
        data = json.loads(fetch("https://registry.npmjs.org/" + name.replace("/", "%2F")))
        return data.get("time", {}).get("created", "")
    if ecosystem == "crates":
        data = json.loads(fetch(f"https://crates.io/api/v1/crates/{urllib.parse.quote(name)}"))
        return data["crate"]["created_at"]
    if ecosystem == "go":
        base = f"https://proxy.golang.org/{go_escape(name)}/@v/"
        versions = [v for v in fetch(base + "list").decode().split() if v]
        if versions:
            info = json.loads(fetch(base + sorted(versions, key=semver_key)[0] + ".info"))
        else:
            info = json.loads(fetch(f"https://proxy.golang.org/{go_escape(name)}/@latest"))
        return info["Time"]
    raise ValueError(ecosystem)


def lookup(ecosystem, name, rows):
    key = (ecosystem, name, SNAPSHOT)
    if key in rows:
        return rows[key]
    if os.environ.get("OFFLINE"):
        return ("unknown", "")
    try:
        result = ("present", first_release(ecosystem, name))
    except urllib.error.HTTPError as error:
        result = ("absent", "") if error.code in (404, 410) else ("unknown", f"http {error.code}")
    except Exception as error:
        result = ("unknown", type(error).__name__)
    if result[0] != "unknown":
        rows[key] = result
        with open(CACHE, "a") as handle:
            handle.write(f"{ecosystem}\t{name}\t{SNAPSHOT}\t{result[0]}\t{result[1]}\n")
    return result


def parse(stamp):
    return datetime.datetime.fromisoformat(stamp.replace("Z", "+00:00"))


def describe(ecosystem, name, commit_date, rows):
    status, first = lookup(ecosystem, name, rows)
    if status != "present" or not first:
        return f"{ecosystem}\t{name}\t{status}\t-\t-"
    if commit_date:
        days = (parse(commit_date) - parse(first)).total_seconds() / 86400
        if days < 0:
            return f"{ecosystem}\t{name}\tabsent-at-commit\t{first}\t{days:.1f}"
        return f"{ecosystem}\t{name}\tpresent\t{first}\t{days:.1f}"
    return f"{ecosystem}\t{name}\tpresent\t{first}\t-"


def main(argv):
    rows = load()
    if len(argv) >= 4 and argv[1] == "lookup":
        print(describe(argv[2], argv[3], argv[4] if len(argv) > 4 else "", rows))
    elif len(argv) == 2 and argv[1] == "batch":
        for line in sys.stdin:
            fields = line.split()
            if len(fields) >= 2:
                print(describe(fields[0], fields[1], fields[2] if len(fields) > 2 else "", rows))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
