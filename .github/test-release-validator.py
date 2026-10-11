#!/usr/bin/env python3
"""Behavioral checks for validate-release-pr.py."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
VALIDATOR = ROOT / ".github" / "validate-release-pr.py"

FILES = {
    "Cargo.toml": '[package]\nname = "klin"\nversion = "{version}"\n',
    "Cargo.lock": '[[package]]\nname = "klin"\nversion = "{version}"\n',
    "README.md": (
        "uses: brajevicm/klin@v{version}\n"
        "git clone --branch v{version} https://github.com/brajevicm/klin\n"
    ),
    ".claude-plugin/marketplace.json": '{"plugins": [{"source": {"ref": "v{version}"}}]}\n',
    ".agents/plugins/marketplace.json": '{"plugins": [{"source": {"ref": "v{version}"}}]}\n',
    "plugins/klin/.claude-plugin/plugin.json": '{"version": "{version}"}\n',
    "plugins/klin/.cursor-plugin/plugin.json": '{"version": "{version}"}\n',
    "CHANGELOG.md": "# Changelog\n\n## {heading}\n\n{notes}## 0.4.1\n\n- Older.\n",
}

NOTES = "### What changes for you\n\n- A new check starts green.\n\n"


def git(cwd: Path, *args: str) -> str:
    done = subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    return done.stdout.strip()


def write_version(
    cwd: Path, version: str, heading: str, notes: str = NOTES, crlf_readme: bool = False
) -> None:
    for relative, template in FILES.items():
        path = cwd / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        data = (
            template.replace("{version}", version)
            .replace("{heading}", heading)
            .replace("{notes}", notes)
            .encode()
        )
        if crlf_readme and relative == "README.md":
            data = data.replace(b"\n", b"\r\n")
        path.write_bytes(data)


def fixture(
    crlf_readme: bool = False,
    executable_readme: bool = False,
    notes: str = NOTES,
    heading: str = "0.5.0",
) -> tuple[Path, str, str]:
    cwd = Path(tempfile.mkdtemp(prefix="klin-release-validator-"))
    git(cwd, "init", "-q")
    git(cwd, "config", "user.name", "klin")
    git(cwd, "config", "user.email", "klin@example.invalid")
    git(cwd, "config", "core.autocrlf", "false")

    write_version(cwd, "0.4.2", "Unreleased", notes)
    git(cwd, "add", ".")
    git(cwd, "commit", "-qm", "base")
    base = git(cwd, "rev-parse", "HEAD")

    write_version(cwd, "0.5.0", heading, notes, crlf_readme=crlf_readme)
    if executable_readme:
        os.chmod(cwd / "README.md", 0o755)
    git(cwd, "add", ".")
    git(cwd, "commit", "-qm", "release")
    return cwd, base, git(cwd, "rev-parse", "HEAD")


def validate(cwd: Path, base: str, head: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, str(VALIDATOR), base, head, "0.5.0"],
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def main() -> None:
    clean = fixture()
    if validate(*clean).returncode != 0:
        raise SystemExit("exact version-only transformation was rejected")

    crlf = fixture(crlf_readme=True)
    if validate(*crlf).returncode == 0:
        raise SystemExit("line-ending change was accepted")

    mode = fixture(executable_readme=True)
    if validate(*mode).returncode == 0:
        raise SystemExit("file-mode change was accepted")

    for label, case, reason in [
        ("a release without notes for the user", fixture(notes=""), b"What changes for you"),
        (
            "notes without a What changes for you section",
            fixture(notes="- A new check starts green.\n\n"),
            b"What changes for you",
        ),
        (
            "an empty What changes for you section",
            fixture(notes="### What changes for you\n\n### Fixes\n\n- A fix.\n\n"),
            b"What changes for you",
        ),
        (
            "notes that hold only a comment",
            fixture(notes="### What changes for you\n\n<!-- TODO: write release notes -->\n\n"),
            b"What changes for you",
        ),
        (
            "notes that hold only an empty bullet",
            fixture(notes="### What changes for you\n\n- \n\n"),
            b"What changes for you",
        ),
        (
            "a release that keeps the Unreleased heading",
            fixture(heading="Unreleased"),
            b"unexpected files",
        ),
    ]:
        if reason not in validate(*case).stderr:
            raise SystemExit(f"{label} was not rejected for its notes")

    grouped = fixture(
        notes="### What changes for you\n\n#### New checks\n\n- `stubs` starts green.\n\n"
    )
    if validate(*grouped).returncode != 0:
        raise SystemExit("notes grouped under H4 headings were rejected")

    print("release validator rejects byte and mode drift and silent release notes")


if __name__ == "__main__":
    main()
