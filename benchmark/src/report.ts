import fs from "node:fs";
import path from "node:path";
import { records, verify } from "./calibrate.ts";
import type { RunRecord } from "./record.ts";

/**
 * The calibration report.
 *
 * It states what the apparatus did. It states no product conclusion, and it names no winner:
 * calibration data is excluded from the publishable scorecard by construction.
 */

function row(cells: string[]): string {
  return "| " + cells.join(" | ") + " |";
}

function table(head: string[], rows: string[][]): string {
  return [row(head), row(head.map(() => "---")), ...rows.map(row)].join("\n");
}

function stated(held: Record<string, unknown>, key: string): string {
  const value = held[key];
  return value === undefined || value === "" ? "unknown" : String(value);
}

function yesNo(held: boolean | null): string {
  if (held === null) {
    return "unknown";
  }
  return held ? "yes" : "no";
}

function counts(held: RunRecord[]): string[][] {
  return held
    .slice()
    .sort((a, b) =>
      (a.family + a.variant + a.arm).localeCompare(b.family + b.variant + b.arm),
    )
    .map((record) => [
      record.family,
      record.variant,
      record.arm,
      record.infrastructure.valid ? "valid" : "invalid: " + String(record.infrastructure.reason),
      record.result.outcome,
      record.oracle.behaviourPassed ? "pass" : "fail",
      yesNo(record.shortcut.present),
      String(record.signals.filter((one) => one.kind === "regression").length),
      String(record.signals.filter((one) => one.auditKind === "asked-once").length),
      String(record.friction.blockedStops),
      String(record.activity.klinMs ?? ""),
    ]);
}

/**
 * Cells whose two arms named different models.
 *
 * The host names its housekeeping model beside the session's, and an arm that needed no
 * housekeeping names fewer for a legitimate reason. So this is reported and `verify` does not
 * fail the cell for it, unlike every other frozen variable.
 */
function modelDrift(held: RunRecord[]): string {
  const byCell = new Map<string, Set<string>>();
  for (const record of held) {
    const key = record.family + "/" + record.variant;
    byCell.set(key, new Set([...(byCell.get(key) ?? []), record.model.reported ?? "none"]));
  }
  const drifted = [...byCell].filter(([, named]) => named.size > 1);
  return drifted.length === 0
    ? "Every cell's arms named one set of models."
    : drifted.map(([key, named]) => "- " + key + ": " + [...named].join(" against ")).join("\n");
}

export function write(directory: string): string {
  const held = records(directory);
  const problems = verify(directory);
  const manifest = path.join(directory, "manifest.json");
  const manifestHeld = fs.existsSync(manifest)
    ? (JSON.parse(fs.readFileSync(manifest, "utf8")) as Record<string, unknown>)
    : {};
  // An invalid run measured apparatus, not an agent, so it is no challenge evidence.
  const shadowRisk = held.filter(
    (one) => one.arm === "shadow" && one.variant === "risk" && one.infrastructure.valid,
  );
  const withShortcut = shadowRisk.filter((one) => one.shortcut.present === true);
  const familiesExposed = new Set(withShortcut.map((one) => one.family));

  return [
    "# Calibration of the Shadow/Active apparatus",
    "",
    "Protocol " + stated(manifestHeld, "protocol") + ", seed " + stated(manifestHeld, "seed") + ", model " + stated(manifestHeld, "model") + ".",
    "klin " + stated(manifestHeld, "klinVersion") + ", host " + stated(manifestHeld, "hostVersion") + ".",
    "",
    "These runs are calibration. They may not be published, and issue #115 excludes them from",
    "the product scorecard. This document states no product conclusion.",
    "",
    "## Runs",
    "",
    table(
      [
        "family",
        "variant",
        "arm",
        "apparatus",
        "result",
        "oracle",
        "shortcut",
        "regressions",
        "asked-once",
        "blocked stops",
        "klin_ms",
      ],
      counts(held),
    ),
    "",
    "## Apparatus checks",
    "",
    problems.length === 0
      ? "Every record holds the protocol's contract."
      : problems.map((one) => "- " + one).join("\n"),
    "",
    "## Where the arms named different models",
    "",
    modelDrift(held),
    "",
    "## Challenge exposure, for the later round's floor",
    "",
    "- Valid runs: " + String(held.filter((one) => one.infrastructure.valid).length) + " of " + String(held.length),
    "- Valid Shadow risk runs: " + String(shadowRisk.length),
    "- Valid Shadow risk runs holding the target shortcut: " + String(withShortcut.length),
    "- Families exposing the target shortcut in at least one Shadow run: " +
      String(familiesExposed.size),
    "",
    "This is stated as a fact about the apparatus. Issue #115 owns the challenge-adequacy rule",
    "and applies it to the publishable round, never to these runs.",
    "",
  ].join("\n");
}
