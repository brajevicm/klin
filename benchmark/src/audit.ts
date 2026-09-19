import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as evidence from "./evidence.ts";
import { families, VARIANTS, type Variant } from "./catalogue.ts";
import { detect, type Finding } from "./detectors.ts";
import * as paths from "./paths.ts";
import type { RunRecord } from "./record.ts";
import { klinVersion } from "./session.ts";
import { sourceCommit } from "./trial.ts";
import { copyTree, files, overlay, sha256 } from "./trees.ts";
import * as workspace from "./workspace.ts";

type Status = "PASS" | "FAIL" | "ERR" | "MISSING";

interface Site {
  file: string;
  line: number;
}

interface Verdict {
  caught: boolean | null;
  label: string;
  sites: Site[];
  delivery: "delivered" | "would-have-been-delivered" | "direct" | "recorded";
  resolved?: boolean;
}

interface Row {
  subject: string;
  family: string;
  variant: string;
  arm: string;
  detector: Verdict;
  whole: Verdict;
  signal: Verdict;
}

type Disagreement =
  | "resolved-signal"
  | "hook-only-review"
  | "gate-gap"
  | "changed-window-gap"
  | "signal-mismatch";

interface Provenance {
  commit: string;
  version: string;
  binarySha256: string;
}

interface HarnessProvenance {
  commit: string;
  dirty: boolean;
}

interface Metadata {
  evidenceKlin: Provenance;
  evidenceHarness: HarnessProvenance;
  auditKlin: Provenance;
  auditHarness: HarnessProvenance;
}

interface Report {
  gates?: { name?: unknown; status?: unknown }[];
  findings?: Record<string, unknown>[];
}

function json<T>(file: string): T {
  return JSON.parse(fs.readFileSync(file, "utf8")) as T;
}

function regular(file: string): boolean {
  try {
    return fs.lstatSync(file).isFile();
  } catch {
    return false;
  }
}

function status(value: unknown): Status {
  switch (String(value ?? "").trim().toUpperCase()) {
    case "OK":
    case "PASS":
      return "PASS";
    case "FAIL":
      return "FAIL";
    case "ERR":
      return "ERR";
    default:
      return "MISSING";
  }
}

function caught(value: Status): boolean | null {
  return value === "FAIL" ? true : value === "PASS" ? false : null;
}

function verdict(value: Status, sites: Site[], delivery: Verdict["delivery"]): Verdict {
  return { caught: caught(value), label: value, sites, delivery };
}

function detectorVerdict(present: boolean | null, sites: Site[]): Verdict {
  return {
    caught: present,
    label: present === true ? "FOUND" : present === false ? "PASS" : "UNKNOWN",
    sites,
    delivery: "recorded",
  };
}

function rowFor(text: string, gate: string): Status {
  const safe = gate.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = text.match(new RegExp("^\\s{2}(ok|FAIL|ERR)\\s+" + safe + "\\s*$", "m"));
  return status(match?.[1]);
}

function numberOf(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function textOf(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function lineAt(root: string, file: string, wanted: string): number | null {
  const at = path.join(root, file);
  if (!regular(at)) {
    return null;
  }
  const lines = fs.readFileSync(at, "utf8").split("\n");
  const found = lines.findIndex((line) => line.includes(wanted));
  return found < 0 ? null : found + 1;
}

function detectorSite(site: Record<string, unknown>, base: string, final: string): Site | null {
  const file = textOf(site.file) ?? textOf(site.document) ?? textOf(site.path);
  const line = numberOf(site.line);
  if (file && line !== null && line > 0) {
    return { file, line };
  }
  const wanted =
    textOf(site.cites) ??
    textOf(site.function) ??
    textOf(site.symbol) ??
    textOf(site.test) ??
    textOf(site.dependency) ??
    textOf(site.item) ??
    textOf(site.line);
  if (file && wanted) {
    const found = lineAt(final, file, wanted) ?? lineAt(base, file, wanted);
    if (found !== null) {
      return { file, line: found };
    }
  }
  if (wanted) {
    for (const tree of [final, base]) {
      for (const relative of files(tree)) {
        const found = lineAt(tree, relative, wanted);
        if (found !== null) {
          return { file: relative, line: found };
        }
      }
    }
  }
  if (file) {
    return { file, line: 0 };
  }
  return null;
}

function detectorSites(found: { sites: unknown[] }, base: string, final: string): Site[] {
  return found.sites
    .map((site) =>
      site !== null && typeof site === "object"
        ? detectorSite(site as Record<string, unknown>, base, final)
        : null,
    )
    .filter((site): site is Site => site !== null);
}

function findingSites(found: Record<string, unknown>[], base: string, final: string): Site[] {
  return found
    .map((one) => {
      const file = textOf(one.file);
      const line = numberOf(one.line);
      if (!file) {
        return null;
      }
      if (line !== null && line > 0) {
        return { file, line };
      }
      const wanted = textOf(one.text);
      const foundAt = wanted && (lineAt(final, file, wanted) ?? lineAt(base, file, wanted));
      return { file, line: foundAt ?? 0 };
    })
    .filter((site): site is Site => site !== null);
}

function hookSites(text: string): Site[] {
  const found: Site[] = [];
  const pattern = /(?:^|\s)((?:[A-Za-z0-9_.-]+\/)*[A-Za-z0-9_.-]+):(\d+)\b/gm;
  for (const match of text.matchAll(pattern)) {
    found.push({ file: match[1], line: Number(match[2]) });
  }
  return found;
}

function uniqueSites(sites: Site[]): Site[] {
  const seen = new Set<string>();
  return sites.filter((site) => {
    const key = site.file + ":" + String(site.line);
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function binary(): string {
  return path.resolve(process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin"));
}

function clear(root: string): void {
  for (const entry of fs.readdirSync(root)) {
    if (entry !== ".git") {
      fs.rmSync(path.join(root, entry), { recursive: true, force: true });
    }
  }
}

function repository(base: string, final: string, root: string): string {
  const repo = path.join(root, "repo");
  fs.rmSync(repo, { recursive: true, force: true });
  copyTree(base, repo);
  workspace.git(repo, "init", "--quiet");
  workspace.git(repo, "add", "-A");
  workspace.git(repo, "commit", "--quiet", "-m", "the starting tree");
  clear(repo);
  copyTree(final, repo);
  return repo;
}

function runWhole(base: string, final: string, gate: string, root: string): Verdict {
  const repo = repository(base, final, root);
  const state = path.join(root, "whole-state");
  const ran = spawnSync(binary(), ["gate", "--json"], {
    cwd: repo,
    encoding: "utf8",
    env: { ...process.env, KLIN_STATE_DIR: state },
    maxBuffer: 16 * 1024 * 1024,
  });
  if (ran.error) {
    throw new Error("the whole run could not start: " + ran.error.message);
  }
  let report: Report;
  try {
    report = JSON.parse(ran.stdout ?? "") as Report;
  } catch {
    throw new Error("the whole run did not print JSON:\n" + (ran.stdout || ran.stderr));
  }
  const gateRow = (report.gates ?? []).find((one) => one.name === gate);
  const findings = (report.findings ?? []).filter((one) => one.gate === gate);
  return verdict(status(gateRow?.status), findingSites(findings, base, final), "recorded");
}

function runHook(base: string, final: string, gate: string, root: string): Verdict {
  const repo = repository(base, final, root);
  const ran = spawnSync(binary(), ["gate", "--hook", "--changed"], {
    cwd: repo,
    input: JSON.stringify({ hook_event_name: "Stop", session_id: "benchmark-audit" }),
    encoding: "utf8",
    env: { ...process.env, KLIN_STATE_DIR: path.join(root, "hook-state") },
    maxBuffer: 16 * 1024 * 1024,
  });
  if (ran.error) {
    throw new Error("the hook run could not start: " + ran.error.message);
  }
  const output = (ran.stdout ?? "") + "\n" + (ran.stderr ?? "");
  return verdict(rowFor(output, gate), hookSites(output), "direct");
}

function recordedSignals(record: RunRecord, gate: string, base: string, final: string): Verdict {
  const signals = record.signals.filter((one) => one.gate === gate);
  const sites = signals
    .filter((one) => one.file && one.line !== null)
    .map((one) => {
      const file = one.file as string;
      const line = one.line as number;
      if (line > 0) {
        return { file, line };
      }
      const wanted = one.text && one.text !== "file" ? one.text : null;
      return { file, line: wanted ? (lineAt(final, file, wanted) ?? lineAt(base, file, wanted) ?? 0) : 0 };
    });
  const delivery = record.arm === "active" ? "delivered" : "would-have-been-delivered";
  return {
    caught: signals.length > 0,
    label: signals.length > 0 ? "FOUND" : "PASS",
    sites: uniqueSites(sites),
    delivery,
    resolved: signals.some((one) => one.outcome === "fixed-next"),
  };
}

function rowDisagrees(one: Row): boolean {
  const values = [one.detector.caught, one.whole.caught, one.signal.caught];
  if (values.some((value) => value === null)) {
    return false;
  }
  return new Set(values).size > 1;
}

function resolvedSignal(one: Row): boolean {
  return (
    one.signal.resolved === true &&
    one.detector.caught === false &&
    one.whole.caught === false &&
    one.signal.caught === true
  );
}

function hookOnlyReview(one: Row): boolean {
  return (
    one.family === "inventory" &&
    one.arm === "-" &&
    one.signal.delivery === "direct" &&
    one.detector.caught === true &&
    one.whole.caught === false &&
    one.signal.caught === true
  );
}

function disagreement(one: Row): Disagreement | null {
  if (!rowDisagrees(one)) {
    return null;
  }
  if (resolvedSignal(one)) {
    return "resolved-signal";
  }
  if (hookOnlyReview(one)) {
    return "hook-only-review";
  }
  if (one.whole.caught === false && one.detector.caught === true) {
    return "gate-gap";
  }
  if (one.whole.caught === true && one.signal.caught === false) {
    return "changed-window-gap";
  }
  if (one.detector.caught === false && one.signal.caught === true) {
    return "signal-mismatch";
  }
  throw new Error(one.subject + " has an unclassifiable disagreement");
}

function siteFor(one: Row, kind: Disagreement): string {
  const sites =
    kind === "resolved-signal" || kind === "signal-mismatch"
      ? [...one.signal.sites, ...one.detector.sites, ...one.whole.sites]
      : kind === "changed-window-gap"
        ? [...one.whole.sites, ...one.signal.sites, ...one.detector.sites]
        : [...one.detector.sites, ...one.signal.sites, ...one.whole.sites];
  const site = uniqueSites(sites)[0];
  if (!site) {
    throw new Error(one.subject + " " + kind + " has no file and line");
  }
  return site.file + ":" + String(site.line);
}

function display(one: Verdict): string {
  if (one.delivery === "direct") {
    return one.label + "/direct";
  }
  return one.label + "/" + one.delivery;
}

function markdown(setId: string, rows: Row[], metadata: Metadata): string {
  const disagreements = rows.filter(rowDisagrees);
  const groups = new Map<Disagreement, Row[]>();
  for (const row of disagreements) {
    const kind = disagreement(row);
    if (kind === null) {
      throw new Error(row.subject + " was listed as a disagreement but has no disagreement class");
    }
    groups.set(kind, [...(groups.get(kind) ?? []), row]);
  }
  const table = [
    "| subject | family | variant | arm | detector | whole | recorded signal | disagreement | site |",
    "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ...rows.map((row) => {
      const kind = disagreement(row);
      return "| " + [
        row.subject,
        row.family,
        row.variant,
        row.arm,
        row.detector.label,
        row.whole.label,
        display(row.signal),
        kind ?? "-",
        kind === null ? "-" : siteFor(row, kind),
      ].join(" | ") + " |";
    }),
  ];
  const section = (kind: Disagreement, reading: string): string[] => {
    const held = groups.get(kind) ?? [];
    const sites = [...new Set(held.map((row) => siteFor(row, kind)))].join(", ");
    const count = String(held.length);
    let paragraph: string;
    if (held.length === 0) {
      paragraph = "No " + reading.toLowerCase() + " appear in these rows.";
    } else {
      switch (kind) {
        case "resolved-signal":
          paragraph = count + " resolved-signal row(s) have a historical signal with outcome `fixed-next`, while the frozen detector and current whole run are clean. The disagreement sites are " + sites + "; these are successful feedback episodes, not noise candidates.";
          break;
        case "hook-only-review":
          paragraph = count + " hook-only-review row(s) are inventory exemplars where the whole run is intentionally non-blocking but the direct hook asks or blocks. The disagreement sites are " + sites + "; this is expected inventory policy, not a production gate gap.";
          break;
        case "gate-gap":
          paragraph = count + " gate-gap row(s) have a detector finding while the whole production run stayed quiet. The disagreement sites are " + sites + "; these are production gate coverage gaps.";
          break;
        case "changed-window-gap":
          paragraph = count + " changed-window-gap row(s) have a whole-run finding with no recorded signal for that gate. The disagreement sites are " + sites + "; the changed-files window missed a finding the full run sees.";
          break;
        case "signal-mismatch":
          paragraph = count + " signal-mismatch row(s) have a recorded signal that the frozen detector does not report and no `fixed-next` resolution or expected hook policy explains. The disagreement sites are " + sites + "; these need follow-up before they are treated as product defects.";
          break;
      }
    }
    return [
      "### " + reading,
      "",
      paragraph,
      "",
    ];
  };
  return [
    "# Benchmark audit, " + setId,
    "",
    "The audit uses the frozen `record.shortcut` detector verdict for every valid recorded run, and runs the current production binary over each recorded final tree and each fixture's `bad/` exemplar. `whole` is `klin gate --json`; `recorded signal` is the historical signal rows in the run (`delivered` in Active and `would-have-been-delivered` in Shadow), while exemplar rows use a direct current hook invocation.",
    "",
    "## Provenance",
    "",
    "- Frozen evidence klin: `" + metadata.evidenceKlin.version + "`, commit `" + metadata.evidenceKlin.commit + "`, binary SHA-256 `" + metadata.evidenceKlin.binarySha256 + "`.",
    "- Frozen evidence harness: commit `" + metadata.evidenceHarness.commit + "` (" + (metadata.evidenceHarness.dirty ? "dirty" : "clean") + ").",
    "- Audit klin: `" + metadata.auditKlin.version + "`, commit `" + metadata.auditKlin.commit + "`, binary SHA-256 `" + metadata.auditKlin.binarySha256 + "`.",
    "- Audit harness: commit `" + metadata.auditHarness.commit + "` (" + (metadata.auditHarness.dirty ? "dirty" : "clean") + ").",
    "",
    "- Rows: " + String(rows.length),
    "- Disagreements: " + String(disagreements.length),
    "",
    ...table,
    "",
    "## Reading the disagreements",
    "",
    ...section("resolved-signal", "Resolved signals"),
    ...section("hook-only-review", "Hook-only review"),
    ...section("gate-gap", "Gate gaps"),
    ...section("changed-window-gap", "Changed-window gaps"),
    ...section("signal-mismatch", "Signal mismatches"),
  ].join("\n");
}

function metadataOf(runs: RunRecord[]): Metadata {
  const frozen = runs[0];
  if (!frozen) {
    throw new Error("the evidence set has no valid runs");
  }
  const klin = binary();
  const binarySha256 = sha256(fs.readFileSync(klin));
  return {
    evidenceKlin: frozen.klin,
    evidenceHarness: { commit: frozen.harness.commit, dirty: frozen.harness.dirty },
    auditKlin: {
      commit: sourceCommit(klin, binarySha256),
      version: klinVersion(klin),
      binarySha256,
    },
    auditHarness: {
      commit: workspace.git(paths.REPO, "rev-parse", "HEAD"),
      dirty: workspace.git(paths.REPO, "status", "--porcelain") !== "",
    },
  };
}

function records(root: string): RunRecord[] {
  return fs
    .readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(root, entry.name, "record.json"))
    .filter(regular)
    .map((file) => json<RunRecord>(file))
    .filter((record) => record.infrastructure?.valid === true)
    .sort((a, b) => a.order - b.order || a.trialId.localeCompare(b.trialId));
}

function badTree(variant: Variant, root: string): { base: string; final: string } {
  const base = workspace.startingTree(variant, path.join(root, "base"));
  const final = path.join(root, "final");
  copyTree(base, final);
  overlay(path.join(variant.root, "bad"), final);
  return { base, final };
}

function archiveFor(directory: string, explicit: string): string {
  if (explicit !== "") {
    return path.resolve(explicit);
  }
  const descriptor = json<{ archive?: string }>(path.join(directory, "evidence.json"));
  if (!descriptor.archive) {
    throw new Error("the evidence set states no raw archive; pass --archive FILE");
  }
  return path.join(path.dirname(directory), descriptor.archive);
}

export function write(directory: string, archive: string): string {
  const evidenceRoot = path.resolve(directory);
  const archiveFile = archiveFor(evidenceRoot, archive);
  const problems = evidence.verify(evidenceRoot, archiveFile);
  if (problems.length > 0) {
    throw new Error("evidence is not intact:\n" + problems.join("\n"));
  }
  const extracted = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-audit-"));
  try {
    const ran = spawnSync("tar", ["-xzf", archiveFile, "-C", extracted], { encoding: "utf8" });
    if (ran.status !== 0) {
      throw new Error("the raw archive could not be extracted: " + (ran.stderr ?? ran.stdout ?? ""));
    }
    const found = families();
    const runs = records(extracted);
    const metadata = metadataOf(runs);
    const rows: Row[] = [];
    for (const record of runs) {
      const family = found[record.family];
      const variant = family?.variants[record.variant as keyof typeof family.variants];
      if (!family || !variant) {
        throw new Error(record.trialId + " names no catalogue family/variant");
      }
      const base = path.join(extracted, record.trialId, "fixtures", "base");
      const final = path.join(extracted, record.trialId, "fixtures", "final");
      if (!fs.existsSync(base) || !fs.existsSync(final)) {
        throw new Error(record.trialId + " has no recorded base and final trees");
      }
      const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-audit-run-"));
      try {
        const whole = runWhole(base, final, record.gate, room);
        const signal = recordedSignals(record, record.gate, base, final);
        rows.push({
          subject: "run/" + record.trialId,
          family: record.family,
          variant: record.variant,
          arm: record.arm,
          detector: detectorVerdict(record.shortcut.present, detectorSites(record.shortcut, base, final)),
          whole,
          signal,
        });
      } finally {
        fs.rmSync(room, { recursive: true, force: true });
      }
    }
    for (const family of Object.values(found)) {
      for (const variantName of VARIANTS) {
        const variant = family.variants[variantName];
        const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-audit-exemplar-"));
        try {
          const trees = badTree(variant, room);
          const detected = detect(variant.shortcut, trees.base, trees.final);
          const whole = runWhole(trees.base, trees.final, family.spec.gate, room);
          const signal = runHook(trees.base, trees.final, family.spec.gate, room);
          rows.push({
            subject: "exemplar/" + family.name + "/" + variantName,
            family: family.name,
            variant: variantName,
            arm: "-",
            detector: detectorVerdict(detected.present, detectorSites(detected, trees.base, trees.final)),
            whole,
            signal,
          });
        } finally {
          fs.rmSync(room, { recursive: true, force: true });
        }
      }
    }
    return markdown(path.basename(evidenceRoot), rows, metadata);
  } finally {
    fs.rmSync(extracted, { recursive: true, force: true });
  }
}
