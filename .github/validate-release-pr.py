#!/usr/bin/env python3
"""Prove that a release PR is only the cargo-release version transformation."""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib

EXPECTED = [
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
    "Cargo.lock",
    "Cargo.toml",
    "README.md",
    "plugins/klin/.claude-plugin/plugin.json",
    "plugins/klin/.cursor-plugin/plugin.json",
]


def fail(message: str) -> None:
    raise SystemExit(message)


def git(*args: str) -> str:
    done = subprocess.run(
        ["git", *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if done.returncode != 0:
        fail(done.stderr.strip() or f"git {' '.join(args)} failed")
    return done.stdout


def show(ref: str, path: str) -> str:
    return git("show", f"{ref}:{path}")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        fail(f"{label}: expected exactly one {old!r}, found {count}")
    return text.replace(old, new, 1)


def package_version(cargo_toml: str) -> str:
    held = tomllib.loads(cargo_toml)["package"]["version"]
    if not isinstance(held, str):
        fail("Cargo.toml package.version is not a string")
    return held


def normalize(path: str, text: str, current: str, previous: str) -> str:
    if path == "Cargo.toml":
        return replace_once(
            text,
            f'version = "{current}"',
            f'version = "{previous}"',
            path,
        )
    if path == "Cargo.lock":
        old = f'name = "klin"\nversion = "{current}"'
        new = f'name = "klin"\nversion = "{previous}"'
        return replace_once(text, old, new, path)
    if path.endswith("plugin.json"):
        return replace_once(
            text,
            f'"version": "{current}"',
            f'"version": "{previous}"',
            path,
        )
    if path.endswith("marketplace.json"):
        return replace_once(
            text,
            f'"ref": "v{current}"',
            f'"ref": "v{previous}"',
            path,
        )
    if path == "README.md":
        text = replace_once(
            text,
            f"brajevicm/klin@v{current}",
            f"brajevicm/klin@v{previous}",
            path,
        )
        return replace_once(
            text,
            f"--branch v{current}",
            f"--branch v{previous}",
            path,
        )
    fail(f"no release normalizer for {path}")


def main() -> None:
    if len(sys.argv) != 4:
        fail("usage: validate-release-pr.py BASE HEAD X.Y.Z")

    base, head, version = sys.argv[1:]
    if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is None:
        fail(f"release version must be X.Y.Z, got {version!r}")

    changed = sorted(
        line for line in git("diff", "--name-only", base, head, "--").splitlines() if line
    )
    if changed != EXPECTED:
        fail(
            "release PR changed unexpected files\n"
            f"expected: {EXPECTED}\n"
            f"actual:   {changed}"
        )

    previous = package_version(show(base, "Cargo.toml"))
    current = package_version(show(head, "Cargo.toml"))
    if current != version:
        fail(f"release branch names {version}, but Cargo.toml has {current}")
    if previous == current:
        fail(f"release does not change the version from {previous}")

    for path in EXPECTED:
        before = show(base, path)
        after = show(head, path)
        normalized = normalize(path, after, current, previous)
        if normalized != before:
            fail(
                f"{path}: release PR changes content beyond the expected "
                f"{previous} -> {current} version substitution"
            )

    print(f"release diff is exactly the {previous} -> {current} version transformation")


if __name__ == "__main__":
    main()
