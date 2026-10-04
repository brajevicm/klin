#!/usr/bin/env python3
"""Prepare and validate the preregistered #459 Active/Shadow run plan.

This script is intentionally outcome-blind. It never runs an agent or a detector.
It derives the 72-run order from the frozen protocol and refuses to treat pending
host bindings, project-check discovery, or study-binary provenance as executable.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
POPULATIONS = ROOT / "populations.tsv"
TASKS = ROOT / "tasks.tsv"
RUNTIME = ROOT / "controlled-runtime.tsv"
RUN_PLAN = ROOT / "run-plan.tsv"
FAMILIES = ("OpenAI Codex", "Claude Code")
LANGUAGES = ("Rust", "TypeScript", "Python")
STUDY_COMMIT = "43a139a8a16964a65ceb97b939c3499c9cf90b4d"
STUDY_VERSION = "0.4.1"


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


def runtime(allow_pending: bool) -> dict[str, str]:
    rows = read_tsv(RUNTIME)
    if len(rows) != 1:
        raise SystemExit(f"expected one controlled-runtime row, found {len(rows)}")
    row = rows[0]
    if row["study_commit"] != STUDY_COMMIT or row["klin_version"] != STUDY_VERSION:
        raise SystemExit("controlled-runtime row does not match the frozen study basis")
    state = row["state"]
    if state == "ready":
        if not re.fullmatch(r"[0-9a-f]{64}", row["executable_sha256"]):
            raise SystemExit("ready runtime lacks a lowercase 64-hex executable SHA-256")
        if not row["host_platform"]:
            raise SystemExit("ready runtime lacks host_platform")
    elif state == "pending-local-build":
        if not allow_pending:
            raise SystemExit(
                "study binary is not frozen yet; build study_commit locally and record "
                "the exact executable SHA-256 before opening controlled tasks"
            )
    else:
        raise SystemExit(f"unknown controlled runtime state: {state!r}")
    return row


def bindings(allow_pending: bool) -> dict[tuple[str, str], dict[str, str]]:
    rows = read_tsv(TASKS)
    expected = 9 * len(FAMILIES)
    if len(rows) != expected:
        raise SystemExit(f"expected {expected} task/family binding rows, found {len(rows)}")

    result: dict[tuple[str, str], dict[str, str]] = {}
    by_family: dict[str, list[dict[str, str]]] = {family: [] for family in FAMILIES}
    by_task: dict[str, list[dict[str, str]]] = {}

    for row in rows:
        key = (row["task_id"], row["agent_family"])
        if key in result:
            raise SystemExit(f"duplicate task/family binding: {key}")
        if row["agent_family"] not in FAMILIES:
            raise SystemExit(f"unexpected agent family: {row['agent_family']}")
        result[key] = row
        by_family[row["agent_family"]].append(row)
        by_task.setdefault(row["task_id"], []).append(row)

    # The protocol resolves each host exactly once. Every task row for a family
    # must therefore repeat one identical preflight result; per-task model drift
    # is never allowed.
    for family, family_rows in by_family.items():
        if len(family_rows) != 9:
            raise SystemExit(f"{family}: expected 9 task rows, found {len(family_rows)}")
        signatures = {
            (row["binding_state"], row["model_id"], row["host_version"])
            for row in family_rows
        }
        if len(signatures) != 1:
            raise SystemExit(
                f"{family}: task rows disagree on the one frozen host/model binding"
            )
        state, model_id, host_version = next(iter(signatures))
        if state == "bound":
            if not model_id or not host_version:
                raise SystemExit(f"{family}: bound preflight lacks model/host")
        elif state == "unavailable":
            if model_id or host_version:
                raise SystemExit(f"{family}: unavailable preflight has model/host")
        elif state == "pending-preflight":
            if not allow_pending:
                raise SystemExit(
                    f"{family} is still pending neutral host preflight; "
                    "do not open or run controlled tasks yet"
                )
        else:
            raise SystemExit(f"{family}: unknown binding_state {state!r}")

    # Project-check commands are a task property, not an agent-family property.
    # Both repeated task rows must agree, and every non-explicit workflow-derived
    # set must be frozen before execution.
    for task_id, task_rows in by_task.items():
        if len(task_rows) != len(FAMILIES):
            raise SystemExit(f"{task_id}: expected one row per agent family")
        check_signatures = {
            (row["project_checks_state"], row["project_checks"]) for row in task_rows
        }
        if len(check_signatures) != 1:
            raise SystemExit(f"{task_id}: project-check rows disagree across families")
        check_state, _ = next(iter(check_signatures))
        if check_state.startswith("pending") and not allow_pending:
            raise SystemExit(
                f"{task_id}: base project checks are not frozen; "
                "freeze them before Active/Shadow execution"
            )

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
        help="generate the deterministic plan before preflight/runtime freezing; never use it to execute runs",
    )
    args = parser.parse_args()

    population = controlled_population()
    frozen_runtime = runtime(args.allow_pending)
    bound = bindings(args.allow_pending)
    output: list[dict[str, str | int]] = []
    sequence = 0

    # Family is deliberately the outer loop: the preregistration requires every
    # family to execute as one contiguous study batch.
    for batch_order, family in enumerate(FAMILIES, start=1):
        for task in population:
            binding = bound[(task["population_id"], family)]
            for repetition in (1, 2):
                pair_hash, first, second = arm_order(task["population_id"], family, repetition)
                for order, arm in enumerate((first, second), start=1):
                    sequence += 1
                    output.append(
                        {
                            "sequence": sequence,
                            "batch_order": batch_order,
                            "run_id": f"{task['population_id']}__{slug(family)}__r{repetition}__{arm.lower()}",
                            "task_id": task["population_id"],
                            "repository": task["repository"],
                            "base": task["base"],
                            "agent_family": family,
                            "model_id": binding["model_id"],
                            "host_version": binding["host_version"],
                            "study_executable_sha256": frozen_runtime["executable_sha256"],
                            "repetition": repetition,
                            "arm": arm,
                            "arm_order": order,
                            "pair_sha256": pair_hash,
                            "state": "planned" if binding["binding_state"] == "bound" else binding["binding_state"],
                        }
                    )

    if len(output) != 72 or len({row["run_id"] for row in output}) != 72:
        raise SystemExit("run plan must contain exactly 72 unique rows")

    family_spans = {
        family: [i for i, row in enumerate(output) if row["agent_family"] == family]
        for family in FAMILIES
    }
    for family, indexes in family_spans.items():
        if indexes != list(range(min(indexes), max(indexes) + 1)):
            raise SystemExit(f"{family}: run plan is not one contiguous batch")

    fields = list(output[0])
    with RUN_PLAN.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(output)

    print(f"wrote {len(output)} rows to {RUN_PLAN}")


if __name__ == "__main__":
    main()
