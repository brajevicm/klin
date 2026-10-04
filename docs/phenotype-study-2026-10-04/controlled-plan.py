#!/usr/bin/env python3
"""Prepare and validate the preregistered #459 Active/Shadow run plan.

This script is intentionally outcome-blind. It never runs an agent or a detector.
It derives the 72-run order from the frozen protocol and refuses to treat pending
host bindings as executable.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent
POPULATIONS = ROOT / "populations.tsv"
TASKS = ROOT / "tasks.tsv"
RUN_PLAN = ROOT / "run-plan.tsv"
FAMILIES = ("OpenAI Codex", "Claude Code")
LANGUAGES = ("Rust", "TypeScript", "Python")


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def controlled_population() -> list[dict[str, str]]:
    rows = [row for row in read_tsv(POPULATIONS) if row["kind"] == "controlled"]
    if len(rows) != 9:
        raise SystemExit(f"expected 9 controlled tasks, found {len(rows)}")
    counts = {language: 0 for language in LANGUAGES}
    for row in rows:
        if row["language"] not in counts:
            raise SystemExit(f"unexpected controlled language: {row['language']}")
        counts[row["language"]] += 1
    if any(count != 3 for count in counts.values()):
        raise SystemExit(f"expected three tasks per language, found {counts}")
    return rows


def bindings(allow_pending: bool) -> dict[tuple[str, str], dict[str, str]]:
    rows = read_tsv(TASKS)
    expected = 9 * len(FAMILIES)
    if len(rows) != expected:
        raise SystemExit(f"expected {expected} task/family binding rows, found {len(rows)}")
    result: dict[tuple[str, str], dict[str, str]] = {}
    for row in rows:
        key = (row["task_id"], row["agent_family"])
        if key in result:
            raise SystemExit(f"duplicate task/family binding: {key}")
        if row["agent_family"] not in FAMILIES:
            raise SystemExit(f"unexpected agent family: {row['agent_family']}")
        state = row["binding_state"]
        if state == "bound":
            if not row["model_id"] or not row["host_version"]:
                raise SystemExit(f"bound row lacks model/host: {key}")
        elif state == "unavailable":
            if row["model_id"] or row["host_version"]:
                raise SystemExit(f"unavailable row unexpectedly has model/host: {key}")
        elif state == "pending-preflight":
            if not allow_pending:
                raise SystemExit(
                    f"{key} is still pending neutral host preflight; "
                    "do not open or run controlled tasks yet"
                )
        else:
            raise SystemExit(f"unknown binding_state {state!r} for {key}")
        result[key] = row
    return result


def arm_order(task_id: str, family: str, repetition: int) -> tuple[str, str, str]:
    key = f"357-arm-v1:{task_id}:{family}:{repetition}"
    digest = hashlib.sha256(key.encode("utf-8")).hexdigest()
    active_first = (int(digest[-1], 16) & 1) == 0
    return digest, "Active" if active_first else "Shadow", "Shadow" if active_first else "Active"


def slug(family: str) -> str:
    return family.lower().replace(" ", "-")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--allow-pending",
        action="store_true",
        help="generate the deterministic plan before host preflight; never use it to execute runs",
    )
    args = parser.parse_args()

    population = controlled_population()
    bound = bindings(args.allow_pending)
    output: list[dict[str, str | int]] = []

    for task in population:
        for family in FAMILIES:
            binding = bound[(task["population_id"], family)]
            for repetition in (1, 2):
                pair_hash, first, second = arm_order(task["population_id"], family, repetition)
                for order, arm in enumerate((first, second), start=1):
                    output.append(
                        {
                            "run_id": f"{task['population_id']}__{slug(family)}__r{repetition}__{arm.lower()}",
                            "task_id": task["population_id"],
                            "repository": task["repository"],
                            "base": task["base"],
                            "agent_family": family,
                            "model_id": binding["model_id"],
                            "host_version": binding["host_version"],
                            "repetition": repetition,
                            "arm": arm,
                            "arm_order": order,
                            "pair_sha256": pair_hash,
                            "state": "planned" if binding["binding_state"] == "bound" else binding["binding_state"],
                        }
                    )

    if len(output) != 72 or len({row["run_id"] for row in output}) != 72:
        raise SystemExit("run plan must contain exactly 72 unique runs")

    fields = list(output[0])
    with RUN_PLAN.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(output)

    print(f"wrote {len(output)} rows to {RUN_PLAN}")


if __name__ == "__main__":
    main()
