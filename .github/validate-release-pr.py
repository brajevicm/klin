#!/usr/bin/env python3
"""Prove that a release PR is exactly the cargo-release version transformation."""

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


def git_bytes(*args: str) -> bytes:
    done = subprocess.run(
        ["git", *args],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if done.returncode != 0:
        error = done.stderr.decode("utf-8", errors="replace").strip()
        fail(error or f"git {' '.join(args)} failed")
    return done.stdout


def git_text(*args: str) -> str:
    try:
        return git_bytes(*args).decode("utf-8")
    except UnicodeDecodeError as why:
        fail(f"git {' '.join(args)} returned non-UTF-8 metadata: {why}")


def show(ref: str, path: str) -> bytes:
    return git_bytes("show", f"{ref}:{path}")


def mode(ref: str, path: str) -> str:
    held = git_bytes("ls-tree", "-z", ref, "--", path)
    if not held:
        fail(f"{path}: missing from {ref}")
    return held.split(b" ", 1)[0].decode("ascii")


def replace_once(data: bytes, old: bytes, new: bytes, label: str) -> bytes:
    count = data.count(old)
    if count != 1:
        fail(f"{label}: expected exactly one {old!r}, found {count}")
    return data.replace(old, new, 1)


def package_version(cargo_toml: bytes) -> str:
    try:
        held = tomllib.loads(cargo_toml.decode("utf-8"))["package"]["version"]
    except (UnicodeDecodeError, KeyError, tomllib.TOMLDecodeError) as why:
        fail(f"Cargo.toml package.version cannot be read: {why}")
    if not isinstance(held, str):
        fail("Cargo.toml package.version is not a string")
    return held


def replace_package_version(data: bytes, current: str, previous: str) -> bytes:
    lines = data.splitlines(keepends=True)
    wanted = f'version = "{current}"'.encode()
    replacement = f'version = "{previous}"'.encode()
    in_package = False
    changed = 0

    for index, line in enumerate(lines):
        held = line.rstrip(b"\r\n")
        if held == b"[package]":
            in_package = True
            continue
        if in_package and held.startswith(b"[") and held.endswith(b"]"):
            break
        if in_package and held == wanted:
            lines[index] = replacement + line[len(held) :]
            changed += 1

    if changed != 1:
        fail(f"Cargo.toml: expected one [package] version {current}, found {changed}")
    return b"".join(lines)


def replace_lock_version(data: bytes, current: str, previous: str) -> bytes:
    lines = data.splitlines(keepends=True)
    wanted = f'version = "{current}"'.encode()
    replacement = f'version = "{previous}"'.encode()
    in_klin = False
    changed = 0

    for index, line in enumerate(lines):
        held = line.rstrip(b"\r\n")
        if held == b"[[package]]":
            in_klin = False
            continue
        if held == b'name = "klin"':
            in_klin = True
            continue
        if in_klin and held == wanted:
            lines[index] = replacement + line[len(held) :]
            changed += 1
            in_klin = False

    if changed != 1:
        fail(f"Cargo.lock: expected one klin version {current}, found {changed}")
    return b"".join(lines)


def normalize(path: str, data: bytes, current: str, previous: str) -> bytes:
    if path == "Cargo.toml":
        return replace_package_version(data, current, previous)
    if path == "Cargo.lock":
        return replace_lock_version(data, current, previous)
    if path.endswith("plugin.json"):
        return replace_once(
            data,
            f'"version": "{current}"'.encode(),
            f'"version": "{previous}"'.encode(),
            path,
        )
    if path.endswith("marketplace.json"):
        return replace_once(
            data,
            f'"ref": "v{current}"'.encode(),
            f'"ref": "v{previous}"'.encode(),
            path,
        )
    if path == "README.md":
        data = replace_once(
            data,
            f"brajevicm/klin@v{current}".encode(),
            f"brajevicm/klin@v{previous}".encode(),
            path,
        )
        return replace_once(
            data,
            f"--branch v{current}".encode(),
            f"--branch v{previous}".encode(),
            path,
        )
    fail(f"no release normalizer for {path}")


def changed_files(base: str, head: str) -> list[str]:
    names = git_bytes("diff", "--name-only", "-z", base, head, "--").split(b"\0")
    try:
        return sorted(name.decode("utf-8") for name in names if name)
    except UnicodeDecodeError as why:
        fail(f"release diff contains a non-UTF-8 path: {why}")


def main() -> None:
    if len(sys.argv) != 4:
        fail("usage: validate-release-pr.py BASE HEAD X.Y.Z")

    base, head, version = sys.argv[1:]
    if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is None:
        fail(f"release version must be X.Y.Z, got {version!r}")

    changed = changed_files(base, head)
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
        before_mode = mode(base, path)
        after_mode = mode(head, path)
        if after_mode != before_mode:
            fail(f"{path}: file mode changed from {before_mode} to {after_mode}")

        before = show(base, path)
        after = show(head, path)
        normalized = normalize(path, after, current, previous)
        if normalized != before:
            fail(
                f"{path}: release PR changes bytes beyond the expected "
                f"{previous} -> {current} version substitution"
            )

    print(f"release diff is exactly the {previous} -> {current} version transformation")


if __name__ == "__main__":
    main()
