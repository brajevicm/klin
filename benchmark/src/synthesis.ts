import fs from "node:fs";
import path from "node:path";
import type { Signal } from "./record.ts";
import { sha256 } from "./trees.ts";

export const LOCKED_LABELS_SHA256 = "223bc66b760d18f51a6e4cc2196de5e4d03b2adaf10cc7b997aed24bd13c96dd";
/** The v3 labels, locked in their own commit before the v3 join is read. Null until a person locks them. */
export const LOCKED_V3_LABELS_SHA256: string | null = "c79b438aa9b2e5f95c777ccd534bac2b40fd1fe19166ee44b74e664c09bd8e28";

export const LABELS = ["valid-regression", "valid-review", "undesired"] as const;
export type Label = (typeof LABELS)[number];
type Round = "v1" | "v2" | "v3";

interface Occurrence {
  round: Round;
  trialId: string;
  family: string;
  variant: "risk" | "control";
  arm: "active" | "shadow";
  signal: Signal;
}

interface JoinRow {
  worksheetId: string;
  occurrences: Occurrence[];
}

interface Join {
  evidence: { round: Round; setId: string; manifestSha256: string }[];
  rows: JoinRow[];
}

export interface SiteRow {
  gate: string;
  sites: number;
  "valid-regression": number;
  "valid-review": number;
  undesired: number;
  occurrences: { total: number; "valid-regression": number; "valid-review": number; undesired: number } & Partial<Record<Round, number>>;
}

export interface ArmView {
  runs: number;
  undesired: number;
  controls: number;
  controlsUndesired: number;
  risk: number;
  riskValid: number;
}

export interface ShadowView extends ArmView {
  occurrences: { undesired: number; valid: number };
}

export interface Synthesis {
  labelsSha256: string;
  labels: Record<Label, number>;
  sites: SiteRow[];
  runs: Partial<Record<Round, { active: ArmView; shadow: ShadowView }>>;
}

function fail(message: string): never {
  throw new Error(message);
}

function json<T>(file: string): T {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8")) as T;
  } catch (why) {
    fail(file + " is not valid JSON: " + String(why));
  }
}

function canonical(labels: Record<string, string>): string {
  return JSON.stringify(Object.fromEntries(Object.entries(labels).sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0)))) + "\n";
}

function lockedLabels(directory: string, expected: string): { labels: Record<string, Label>; sha256: string } {
  const file = path.join(directory, "labels.locked.json");
  const bytes = fs.readFileSync(file);
  const digest = sha256(bytes);
  const recorded = fs.readFileSync(path.join(directory, "labels.locked.sha256"), "utf8").trim().split(/\s+/)[0];
  if (digest !== expected || digest !== recorded) {
    fail("labels.locked.json does not match its recorded SHA-256: file " + digest + ", sidecar " + recorded + ", expected " + expected);
  }
  const labels = json<Record<string, string>>(file);
  if (canonical(labels) !== bytes.toString("utf8")) fail("labels.locked.json is not canonical");
  for (const [id, label] of Object.entries(labels)) {
    if (!LABELS.includes(label as Label)) fail(id + " carries an unknown label " + label);
  }
  return { labels: labels as Record<string, Label>, sha256: digest };
}

function manifestRuns(file: string, expected: string): { trialId: string; arm: "active" | "shadow"; variant: "risk" | "control" }[] {
  const bytes = fs.readFileSync(file);
  if (sha256(bytes) !== expected) fail(file + " does not match the manifest the sealed join was built from");
  const read = JSON.parse(bytes.toString("utf8")) as { order?: unknown };
  if (!Array.isArray(read.order)) fail(file + " has no run order");
  return read.order.map((row) => {
    const one = row as Record<string, unknown>;
    if (!["active", "shadow"].includes(String(one.arm)) || !["risk", "control"].includes(String(one.variant))) {
      fail(file + " holds a non-natural run " + String(one.trialId));
    }
    return { trialId: String(one.trialId), arm: one.arm as "active" | "shadow", variant: one.variant as "risk" | "control" };
  });
}

function valid(label: Label): boolean {
  return label !== "undesired";
}

function siteRows(rows: JoinRow[], labels: Record<string, Label>, rounds: Round[]): SiteRow[] {
  const byGate = new Map<string, SiteRow>();
  for (const row of rows) {
    const gates = new Set(row.occurrences.map((one) => one.signal.gate));
    if (gates.size !== 1) fail(row.worksheetId + " joins occurrences of different gates");
    const gate = [...gates][0];
    const label = labels[row.worksheetId];
    const held = byGate.get(gate) ?? {
      gate,
      sites: 0,
      "valid-regression": 0,
      "valid-review": 0,
      undesired: 0,
      occurrences: { total: 0, ...Object.fromEntries(rounds.map((round) => [round, 0])), "valid-regression": 0, "valid-review": 0, undesired: 0 },
    };
    held.sites += 1;
    held[label] += 1;
    held.occurrences.total += row.occurrences.length;
    held.occurrences[label] += row.occurrences.length;
    for (const one of row.occurrences) held.occurrences[one.round] = (held.occurrences[one.round] ?? 0) + 1;
    byGate.set(gate, held);
  }
  return [...byGate.values()].sort((left, right) => left.gate.localeCompare(right.gate));
}

function runViews(
  round: Round,
  runs: { trialId: string; arm: "active" | "shadow"; variant: "risk" | "control" }[],
  rows: JoinRow[],
  labels: Record<string, Label>,
): { active: ArmView; shadow: ShadowView } {
  const seen = new Map<string, Set<Label>>();
  const shadowOccurrences = { undesired: 0, valid: 0 };
  for (const row of rows) {
    for (const one of row.occurrences) {
      if (one.round !== round) continue;
      const wanted = one.arm === "active" ? "delivered" : "would-have-been-delivered";
      if (one.signal.delivery !== wanted) fail(one.trialId + " holds a " + one.arm + " signal with delivery " + String(one.signal.delivery));
      const label = labels[row.worksheetId];
      seen.set(one.trialId, (seen.get(one.trialId) ?? new Set()).add(label));
      if (one.arm === "shadow") shadowOccurrences[valid(label) ? "valid" : "undesired"] += 1;
    }
  }
  const scheduled = new Map(runs.map((one) => [one.trialId, one]));
  for (const row of rows) {
    for (const one of row.occurrences) {
      if (one.round !== round) continue;
      const run = scheduled.get(one.trialId) ?? fail(round + " joins a run the manifest does not schedule: " + one.trialId);
      if (run.arm !== one.arm || run.variant !== one.variant) {
        fail(round + " joins " + one.trialId + " as " + one.arm + "/" + one.variant + " but the manifest schedules " + run.arm + "/" + run.variant);
      }
    }
  }
  const view = (arm: "active" | "shadow"): ArmView => {
    const mine = runs.filter((one) => one.arm === arm);
    const has = (one: { trialId: string }, test: (label: Label) => boolean): boolean => [...(seen.get(one.trialId) ?? [])].some(test);
    return {
      runs: mine.length,
      undesired: mine.filter((one) => has(one, (label) => !valid(label))).length,
      controls: mine.filter((one) => one.variant === "control").length,
      controlsUndesired: mine.filter((one) => one.variant === "control" && has(one, (label) => !valid(label))).length,
      risk: mine.filter((one) => one.variant === "risk").length,
      riskValid: mine.filter((one) => one.variant === "risk" && has(one, valid)).length,
    };
  };
  return { active: view("active"), shadow: { ...view("shadow"), occurrences: shadowOccurrences } };
}

function rate(part: number, whole: number): string {
  return whole === 0 ? "-" : String(part) + "/" + String(whole) + " (" + (100 * part / whole).toFixed(0) + "%)";
}

function markdown(read: Synthesis, join: Join): string {
  const rounds = roundsOf(join);
  const v3 = rounds.includes("v3");
  const out = [
    "# Labeled signal synthesis",
    "",
    "Unblinded after the locked labels verified against SHA-256 `" + read.labelsSha256 + "`.",
    v3
      ? "The population is the v3 paired round only: `" + join.evidence.find((one) => one.round === "v3")?.setId + "`."
      : "The population is the two natural rounds only: v1 `" + join.evidence.find((one) => one.round === "v1")?.setId + "` and v2 `" + join.evidence.find((one) => one.round === "v2")?.setId + "`.",
    "",
    "- Labels: " + LABELS.map((label) => String(read.labels[label]) + " `" + label + "`").join(", "),
    "",
    "## Site-level, per gate",
    "",
    "A site is one worksheet row. Occurrences count every recorded firing that joined to the row, so one site met many times is visible as such.",
    "",
    "| gate | sites | valid-regression | valid-review | undesired | occurrences | " + rounds.join(" | ") + " | occ. valid-regression | occ. valid-review | occ. undesired |",
    "|" + "---|".repeat(9 + rounds.length),
    ...read.sites.map((row) =>
      "| " + [
        row.gate,
        row.sites,
        rate(row["valid-regression"], row.sites),
        rate(row["valid-review"], row.sites),
        rate(row.undesired, row.sites),
        row.occurrences.total,
        ...rounds.map((round) => row.occurrences[round] ?? 0),
        row.occurrences["valid-regression"],
        row.occurrences["valid-review"],
        row.occurrences.undesired,
      ].join(" | ") + " |",
    ),
    "",
    "## Run-level intervention view",
    "",
    v3
      ? "Active counts are delivered signals over the scheduled Active runs of the round. Shadow counts are would-have-been-delivered signals, reported descriptively and never pooled into the Active rates."
      : "Active counts are delivered signals over the scheduled Active natural runs of that round. Shadow counts are would-have-been-delivered signals, reported descriptively and never pooled into the Active rates. v1 and v2 stay separate because their apparatus and fixtures differ.",
    "",
    "| stratum | arm | runs | runs with an undesired signal | controls | controls with an undesired signal | risk runs | risk runs with a valid signal | undesired occurrences | valid occurrences |",
    "|---|---|---|---|---|---|---|---|---|---|",
  ];
  for (const round of rounds) {
    const { active, shadow } = read.runs[round] ?? fail("the synthesis has no " + round + " runs");
    out.push(
      "| " + [round, "active", active.runs, rate(active.undesired, active.runs), active.controls, rate(active.controlsUndesired, active.controls), active.risk, rate(active.riskValid, active.risk), "-", "-"].join(" | ") + " |",
      "| " + [round, "shadow", shadow.runs, rate(shadow.undesired, shadow.runs), shadow.controls, rate(shadow.controlsUndesired, shadow.controls), shadow.risk, rate(shadow.riskValid, shadow.risk), shadow.occurrences.undesired, shadow.occurrences.valid].join(" | ") + " |",
    );
  }
  out.push(
    "",
    "## Interpretation boundary",
    "",
    v3
      ? "These labels judge whether each signal was appropriate when it fired. Guardrails 3 and 4 of `docs/benchmark-rubric-v3.md` read the Active rows above: the runs, and the control runs, that exposed the agent to at least one `undesired` signal. The result document applies the frozen v3 rubric. No admission, natural or seeded record is in this population."
      : "These labels judge whether each signal was appropriate when it fired. They do not cure the challenge-adequacy shortfall of the natural experiment: the repaired v2 round exposed the target shortcut in 3 of 27 Shadow risk runs and one of nine families, below #115's frozen floor, and stays challenge-limited. No seeded (#260/#261) record is in this population or in any rate above; the seeded experiment is a separate conditional-mechanism analysis. This document states counts and turns them into no causal product conclusion beyond what #115's frozen rubric permits.",
    "",
  );
  return out.join("\n");
}

function roundsOf(join: Join): Round[] {
  return join.evidence.map((one) => one.round).sort();
}

export function synthesize(directory: string, manifests: Partial<Record<Round, string>>, expected?: string): Synthesis {
  const rounds = (Object.keys(manifests) as Round[]).sort();
  const names = rounds.join(",");
  if (names !== "v1,v2" && names !== "v3") fail("a synthesis reads v1 and v2 evidence, or v3 evidence alone, and was given " + names);
  const wanted = expected ?? (names === "v3" ? LOCKED_V3_LABELS_SHA256 : LOCKED_LABELS_SHA256);
  if (wanted === null) fail("no locked v3 label hash is recorded, so the v3 join stays sealed");
  const locked = lockedLabels(directory, wanted);
  const join = json<Join>(path.join(directory, "join.sealed.json"));
  if (roundsOf(join).join(",") !== names) fail("the sealed join names the rounds " + roundsOf(join).join(",") + ", not " + names);
  const ids = join.rows.map((row) => row.worksheetId).sort();
  const labeled = Object.keys(locked.labels).sort();
  if (JSON.stringify(ids) !== JSON.stringify(labeled)) fail("the locked labels do not cover exactly the sealed worksheet rows");
  const runs: Synthesis["runs"] = {};
  for (const round of rounds) {
    const evidence = join.evidence.find((one) => one.round === round) ?? fail("the sealed join has no " + round + " evidence");
    const manifest = manifests[round] ?? fail("no " + round + " evidence directory was given");
    runs[round] = runViews(round, manifestRuns(path.join(manifest, "manifest.json"), evidence.manifestSha256), join.rows, locked.labels);
  }
  const labels = { "valid-regression": 0, "valid-review": 0, undesired: 0 };
  for (const label of Object.values(locked.labels)) labels[label] += 1;
  const read: Synthesis = { labelsSha256: locked.sha256, labels, sites: siteRows(join.rows, locked.labels, rounds), runs };
  fs.writeFileSync(path.join(directory, "synthesis.json"), JSON.stringify({ version: 1, status: "unblinded", ...read }, null, 2) + "\n");
  fs.writeFileSync(path.join(directory, "synthesis.md"), markdown(read, join));
  return read;
}
