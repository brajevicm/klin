#!/usr/bin/env python3
"""Prove that release.yml is what dist generates, except that host needs a successful signing job.

dist-workspace.toml sets allow-dirty = ["ci"] for that one edit, which also stops `dist plan`
from reporting any other drift. This check regenerates the workflow without allow-dirty, applies
the one edit, and fails on any other difference. ADR 0070.
"""

from __future__ import annotations

import difflib
import pathlib
import shutil
import subprocess
import sys
import tempfile

WORKFLOW = pathlib.Path(".github/workflows/release.yml")
CONFIG = pathlib.Path("dist-workspace.toml")
ALLOW_DIRTY = 'allow-dirty = ["ci"]\n'
GENERATED = (
    "(needs.custom-sign-release.result == 'skipped' || "
    "needs.custom-sign-release.result == 'success')"
)
EDITED = "needs.custom-sign-release.result == 'success'"


def generated() -> str:
    with tempfile.TemporaryDirectory() as saved:
        shutil.copy(CONFIG, saved)
        shutil.copy(WORKFLOW, saved)
        try:
            config = CONFIG.read_text()
            if config.count(ALLOW_DIRTY) != 1:
                sys.exit(f"{CONFIG} must set {ALLOW_DIRTY.strip()} once.")
            CONFIG.write_text(config.replace(ALLOW_DIRTY, ""))
            subprocess.run(["dist", "generate", "--mode", "ci"], check=True, capture_output=True)
            return WORKFLOW.read_text()
        finally:
            shutil.copy(pathlib.Path(saved) / CONFIG.name, CONFIG)
            shutil.copy(pathlib.Path(saved) / WORKFLOW.name, WORKFLOW)


def main() -> int:
    fresh = generated()
    if fresh.count(GENERATED) != 1:
        print(f"dist no longer generates the host condition {GENERATED!r}; update this check.")
        return 1
    expected = fresh.replace(GENERATED, EDITED)
    committed = WORKFLOW.read_text()
    if committed == expected:
        print(f"{WORKFLOW} matches dist, with host needing a successful signing job.")
        return 0
    diff = difflib.unified_diff(
        expected.splitlines(keepends=True),
        committed.splitlines(keepends=True),
        "expected",
        str(WORKFLOW),
    )
    sys.stdout.writelines(diff)
    print(f"{WORKFLOW} differs from dist output beyond the signing condition.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
