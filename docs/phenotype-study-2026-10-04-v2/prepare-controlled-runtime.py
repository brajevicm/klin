#!/usr/bin/env python3
"""Prepare the exact klin v0.4.2 release binary used by study v2.

Downloads the frozen Apple Silicon release archive, verifies its published
SHA-256, extracts only the klin executable, records the executable SHA-256 in
controlled-runtime.tsv, and leaves the executable at .study-runtime/klin.

This script is outcome-blind and must run before any controlled task is opened.
"""

from __future__ import annotations

import csv
import hashlib
import io
import os
import shutil
import subprocess
import tarfile
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent
RUNTIME = ROOT / "controlled-runtime.tsv"
OUT_DIR = ROOT / ".study-runtime"
OUT_BIN = OUT_DIR / "klin"
ASSET = "klin-aarch64-apple-darwin.tar.xz"
ASSET_SHA256 = "6bca96b0f90bad16bac92c35d3a739ec02f5f915580d92161e35445773468b03"
URL = f"https://github.com/brajevicm/klin/releases/download/v0.4.2/{ASSET}"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_runtime() -> tuple[list[str], dict[str, str]]:
    with RUNTIME.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        rows = list(reader)
        if len(rows) != 1 or reader.fieldnames is None:
            raise SystemExit("expected exactly one controlled-runtime row")
        return reader.fieldnames, rows[0]


def find_klin(archive: tarfile.TarFile) -> tarfile.TarInfo:
    candidates = [
        member
        for member in archive.getmembers()
        if member.isfile() and Path(member.name).name == "klin"
    ]
    if len(candidates) != 1:
        raise SystemExit(f"expected exactly one klin executable in archive, found {len(candidates)}")
    return candidates[0]


def download_asset() -> bytes:
    """Download through authenticated gh first; fall back to public HTTP."""
    gh = shutil.which("gh")
    if gh:
        with tempfile.TemporaryDirectory(prefix="klin-study-release-") as tmp:
            subprocess.run(
                [
                    gh,
                    "release",
                    "download",
                    "v0.4.2",
                    "--repo",
                    "brajevicm/klin",
                    "--pattern",
                    ASSET,
                    "--dir",
                    tmp,
                    "--clobber",
                ],
                check=True,
            )
            downloaded = Path(tmp) / ASSET
            if not downloaded.is_file():
                raise SystemExit(f"gh release download did not produce {ASSET}")
            return downloaded.read_bytes()

    try:
        with urllib.request.urlopen(URL) as response:
            return response.read()
    except urllib.error.HTTPError as exc:
        raise SystemExit(
            f"failed to download {URL}: HTTP {exc.code}. "
            "Install/authenticate GitHub CLI (gh auth login) for a private repository."
        ) from exc


def main() -> None:
    fields, row = read_runtime()
    if row["study_commit"] != "138dc8d0a927c60df289bd485627f472488cf2ba":
        raise SystemExit("runtime study commit does not match study v2")
    if row["klin_version"] != "0.4.2" or row["release_asset"] != ASSET:
        raise SystemExit("runtime release identity does not match study v2")
    if row["release_asset_sha256"] != ASSET_SHA256:
        raise SystemExit("runtime release-asset SHA-256 is not the frozen value")

    payload = download_asset()

    actual_asset_sha = sha256_bytes(payload)
    if actual_asset_sha != ASSET_SHA256:
        raise SystemExit(
            f"release archive SHA-256 mismatch: expected {ASSET_SHA256}, got {actual_asset_sha}"
        )

    with tarfile.open(fileobj=io.BytesIO(payload), mode="r:xz") as archive:
        member = find_klin(archive)
        extracted = archive.extractfile(member)
        if extracted is None:
            raise SystemExit("failed to extract klin from release archive")
        binary = extracted.read()

    executable_sha = sha256_bytes(binary)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    OUT_BIN.write_bytes(binary)
    OUT_BIN.chmod(0o755)

    row["executable_sha256"] = executable_sha
    row["state"] = "ready"
    row["notes"] = (
        f"Verified immutable v0.4.2 release archive {ASSET_SHA256}; "
        f"extracted executable SHA-256 {executable_sha}; runtime path {OUT_BIN.name}."
    )

    with RUNTIME.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerow(row)

    print(f"release asset sha256: {ASSET_SHA256}")
    print(f"klin executable sha256: {executable_sha}")
    print(f"runtime: {OUT_BIN}")


if __name__ == "__main__":
    main()
