import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as evidence from "./evidence.ts";
import type { Signal } from "./record.ts";
import { digest, sha256 } from "./trees.ts";

export interface EvidenceInput {
  name: "v1" | "v2";
  directory: string;
  archive: string;
}

export interface Counts {
  includedRecords: number;
  signalOccurrences: number;
  worksheetRows: number;
}

export interface WorksheetRow {
  worksheetId: string;
  signalCategory: "regression" | "review";
  gate: string;
  taskIntent: string;
  site: { file: string; line: number | null; text: string | null };
  values: unknown;
  context: { path: string; line: number | null; excerpt: string; note: string | null };
  remedy: string | null;
}

export interface Preparation {
  counts: Counts;
  rows: WorksheetRow[];
}

const EXPECTED_SET_IDS = {
  v1: "publishable-2026-09-18",
  v2: "v2-2026-09-20",
} as const;

interface ManifestRow {
  family?: unknown;
  variant?: unknown;
  arm?: unknown;
  order?: unknown;
  repetition?: unknown;
  trialId?: unknown;
}

type RawRecord = Record<string, unknown>;

interface Attempt {
  id: string;
  record: RawRecord | null;
  metadata: RawRecord;
}

interface SelectedRun {
  round: "v1" | "v2";
  row: ManifestRow;
  record: RawRecord;
  raw: string;
  prompt: string;
  base: string;
  signal: Signal;
  signalIndex: number;
  context: WorksheetRow["context"];
}

interface LoadedSet {
  input: EvidenceInput;
  setId: string;
  manifestSha256: string;
  archiveSha256: string;
  archiveBytes: number;
  root: string;
  runs: SelectedRun[];
  selectedRecords: RawRecord[];
}

interface Candidate {
  taskIntent: string;
  signalCategory: WorksheetRow["signalCategory"];
  gate: string;
  file: string;
  line: number | null;
  text: string | null;
  values: unknown;
  remedy: string | null;
  context: WorksheetRow["context"];
  key: string;
}

interface Occurrence {
  candidate: Candidate;
  run: SelectedRun;
}

interface Group {
  candidate: Candidate;
  occurrences: Occurrence[];
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

function text(value: unknown, name: string): string {
  if (typeof value !== "string" || value.length === 0) {
    fail("the worksheet evidence has no " + name);
  }
  return value;
}

function number(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isInteger(value)) {
    fail("the worksheet evidence has no integer " + name);
  }
  return value;
}

function object(value: unknown, name: string): RawRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("the worksheet evidence has no object " + name);
  }
  return value as RawRecord;
}

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as RawRecord)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, held]) => [key, canonical(held)]),
    );
  }
  return value;
}

function stable(value: unknown): string {
  return JSON.stringify(canonical(value));
}

function inside(root: string, file: string): boolean {
  const relative = path.relative(path.resolve(root), path.resolve(file));
  return relative === "" || (!relative.startsWith(".." + path.sep) && !path.isAbsolute(relative));
}

function safeRelative(root: string, relative: string): string {
  const file = path.resolve(root, relative);
  if (!inside(root, file)) fail("the worksheet signal names a path outside its base tree: " + relative);
  return file;
}

function archiveHash(file: string): { sha256: string; bytes: number } {
  if (!fs.existsSync(file) || !fs.statSync(file).isFile()) fail("the raw archive does not exist: " + file);
  const bytes = fs.readFileSync(file);
  return { sha256: sha256(bytes), bytes: bytes.length };
}

function extract(input: EvidenceInput): string {
  const problems = evidence.verify(input.directory, input.archive);
  if (problems.length > 0) {
    fail(input.name + " evidence is not intact:\n" + problems.join("\n"));
  }
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-worksheet-"));
  const ran = spawnSync("tar", ["-xzf", path.resolve(input.archive), "-C", root], {
    encoding: "utf8",
    maxBuffer: 8 * 1024 * 1024,
  });
  if (ran.status !== 0) {
    fs.rmSync(root, { recursive: true, force: true });
    fail(input.name + " raw archive could not be extracted: " + (ran.stderr ?? ran.stdout ?? ""));
  }
  return root;
}

function attempts(directory: string): Map<string, Attempt> {
  const root = path.join(directory, "attempts");
  const found = new Map<string, Attempt>();
  for (const entry of fs.readdirSync(root, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    if (!entry.isDirectory()) continue;
    const at = path.join(root, entry.name);
    const recordFile = path.join(at, "record.json");
    const crashFile = path.join(at, "crash.json");
    const record = fs.existsSync(recordFile) ? json<RawRecord>(recordFile) : null;
    const metadata = record ?? (fs.existsSync(crashFile) ? json<RawRecord>(crashFile) : null);
    if (metadata === null) fail(entry.name + " has neither record.json nor crash.json");
    found.set(entry.name, { id: entry.name, record, metadata });
  }
  return found;
}

function manifest(directory: string): { rows: ManifestRow[]; sha256: string; setId: string } {
  const file = path.join(directory, "manifest.json");
  const read = json<RawRecord>(file);
  if (read.kind !== "publishable" || read.publishable !== true) {
    fail(directory + " is not publishable evidence");
  }
  if (!Array.isArray(read.order) || read.order.length === 0) {
    fail(directory + " has no frozen run order");
  }
  const rows = read.order.map((one) => object(one, "manifest row"));
  const descriptor = json<RawRecord>(path.join(directory, "evidence.json"));
  return {
    rows,
    sha256: sha256(fs.readFileSync(file)),
    setId: text(descriptor.setId, "evidence set id"),
  };
}

function selected(directory: string, rows: ManifestRow[], held: Map<string, Attempt>): { row: ManifestRow; record: RawRecord; id: string }[] {
  const children = new Map<string, Attempt[]>();
  for (const attempt of held.values()) {
    const replaces = attempt.metadata.replaces;
    if (typeof replaces === "string" && replaces.length > 0) {
      children.set(replaces, [...(children.get(replaces) ?? []), attempt]);
    }
  }
  const result: { row: ManifestRow; record: RawRecord; id: string }[] = [];
  const used = new Set<string>();
  for (const row of rows) {
    const original = text(row.trialId, "scheduled trial id");
    let id = original;
    const seen = new Set<string>();
    while (true) {
      if (seen.has(id)) fail(directory + " has a replacement cycle at " + id);
      seen.add(id);
      const attempt = held.get(id);
      if (!attempt) fail(directory + " has no attempt for scheduled trial " + original);
      if (attempt.record !== null && object(attempt.record.infrastructure, id + " infrastructure").valid === true) {
        if ((children.get(id) ?? []).length > 0) fail(directory + " replaces a valid selected attempt " + id);
        if (used.has(id)) fail(directory + " selects attempt " + id + " twice");
        used.add(id);
        const record = attempt.record;
        for (const [name, actual, expected] of [
          ["family", record.family, row.family],
          ["variant", record.variant, row.variant],
          ["arm", record.arm, row.arm],
          ["order", record.order, row.order],
          ["repetition", record.repetition, row.repetition],
        ] as [string, unknown, unknown][]) {
          if (expected !== undefined && actual !== expected) {
            fail(directory + " selected " + id + " with " + name + " " + String(actual) + " instead of " + String(expected));
          }
        }
        if (record.kind !== "publishable" || record.publishable !== true) fail(directory + " selected a non-publishable record " + id);
        if (!["risk", "control"].includes(String(record.variant))) fail(directory + " selected a non-natural record " + id);
        result.push({ row, record, id });
        break;
      }
      const next = children.get(id) ?? [];
      id = next[0].id;
    }
  }
  return result;
}

function promptAt(raw: string, expected: string): string {
  const hooks = path.join(raw, "hooks");
  const prompts: string[] = [];
  for (const directory of fs.readdirSync(hooks).sort()) {
    const payload = path.join(hooks, directory, "payload.json");
    if (!fs.existsSync(payload)) continue;
    const read = json<RawRecord>(payload);
    if (read.hook_event_name === "UserPromptSubmit" && typeof read.prompt === "string") prompts.push(read.prompt);
  }
  const unique = [...new Set(prompts)];
  if (unique.length !== 1) fail(raw + " does not hold exactly one task prompt");
  if (sha256(unique[0]) !== expected) fail(raw + " task prompt does not match the frozen prompt hash");
  return unique[0];
}

function context(base: string, signal: Signal): WorksheetRow["context"] {
  const file = text(signal.file, "signal file");
  const values = signal.values !== null && typeof signal.values === "object" ? signal.values as RawRecord : {};
  let source = file;
  let wantedLine = typeof signal.line === "number" && signal.line > 0 ? signal.line : null;
  if (file === "notes" && typeof values.origin === "string") {
    const found = values.origin.match(/^(.*):(\d+)$/);
    if (found) {
      source = found[1];
      wantedLine = Number(found[2]);
    }
  }
  const at = safeRelative(base, source);
  if (!fs.existsSync(at) || !fs.statSync(at).isFile()) fail("the frozen base has no signal context at " + source);
  const lines = fs.readFileSync(at, "utf8").split(/\r?\n/);
  const wanted = typeof signal.text === "string" && signal.text !== "file" ? signal.text : "";
  const matching = wanted === "" ? -1 : lines.findIndex((line) => line.includes(wanted));
  const exact = matching >= 0;
  const anchor = exact ? matching : wantedLine !== null && wantedLine <= lines.length ? wantedLine - 1 : 0;
  const start = Math.max(0, anchor - 2);
  const shown = lines.slice(start, Math.min(lines.length, start + 7));
  return {
    path: source,
    line: exact || wantedLine !== null ? start + 1 : null,
    excerpt: shown.map((line, index) => String(start + index + 1).padStart(4, "0") + (line === "" ? " |" : " | " + line)).join("\n"),
    note: exact ? null : "The reported signal text is not in the frozen base tree; this is the neutral base context beside the recorded site.",
  };
}

function candidate(run: SelectedRun, signal: Signal, contextValue: WorksheetRow["context"]): Candidate {
  const gate = text(signal.gate, "signal gate");
  const signalCategory = signal.auditKind === "asked-once" ? "review" : "regression";
  if (signalCategory === "review" && signal.kind !== "audit") fail("an asked-once signal is not an audit signal");
  if (signalCategory === "regression" && signal.kind !== "regression") fail("an ordinary audit row reached the worksheet");
  const file = text(signal.file, "signal file");
  const textValue = typeof signal.text === "string" ? signal.text : null;
  const line = typeof signal.line === "number" && signal.line > 0 ? signal.line : null;
  const values = signal.values === undefined ? null : signal.values;
  const remedy = typeof signal.remedy === "string" ? signal.remedy : null;
  const key = stable({
    taskIntent: run.prompt,
    signalCategory,
    gate,
    file,
    text: textValue,
    values,
    remedy,
    context: { path: contextValue.path, excerpt: contextValue.excerpt, note: contextValue.note },
  });
  return {
    taskIntent: run.prompt,
    signalCategory,
    gate,
    file,
    line,
    text: textValue,
    values,
    remedy,
    context: contextValue,
    key,
  };
}

function load(input: EvidenceInput): LoadedSet {
  const details = manifest(input.directory);
  const expectedSetId = EXPECTED_SET_IDS[input.name];
  if (details.setId !== expectedSetId) {
    fail(input.name + " is not the frozen evidence set " + expectedSetId);
  }
  const held = attempts(input.directory);
  const chosen = selected(input.directory, details.rows, held);
  const descriptor = json<RawRecord>(path.join(input.directory, "evidence.json"));
  const archive = archiveHash(input.archive);
  const expectedArchive = text(descriptor.archiveSha256, "archive SHA-256");
  if (
    archive.sha256 !== expectedArchive ||
    archive.bytes !== number(descriptor.archiveBytes, "archive byte count")
  ) {
    fail(input.name + " archive does not match its evidence descriptor");
  }
  const rawRoot = extract(input);
  const runs: SelectedRun[] = [];
  try {
    for (const one of chosen) {
      const raw = path.join(rawRoot, one.id);
      const base = path.join(raw, "fixtures", "base");
      if (!fs.existsSync(base) || digest(base) !== text(object(one.record.fixture, one.id + " fixture").treeSha256, one.id + " base tree hash")) {
        fail(input.name + " selected attempt " + one.id + " has no matching frozen base tree");
      }
      const prompt = promptAt(raw, text(object(one.record.fixture, one.id + " fixture").promptSha256, one.id + " prompt hash"));
      const signals = one.record.signals;
      if (!Array.isArray(signals)) fail(input.name + " selected attempt " + one.id + " has no signal list");
      for (let index = 0; index < signals.length; index += 1) {
        const signal = signals[index] as Signal;
        const selectedRun = { round: input.name, row: one.row, record: one.record, raw, prompt, base, signal, signalIndex: index, context: {} as WorksheetRow["context"] };
        selectedRun.context = context(base, signal);
        runs.push(selectedRun);
      }
    }
    return {
      input,
      setId: details.setId,
      manifestSha256: details.sha256,
      archiveSha256: archive.sha256,
      archiveBytes: archive.bytes,
      root: rawRoot,
      runs,
      selectedRecords: chosen.map((one) => one.record),
    };
  } catch (why) {
    fs.rmSync(rawRoot, { recursive: true, force: true });
    throw why;
  }
}

function occurrenceOrder(left: Occurrence, right: Occurrence): number {
  return left.run.round.localeCompare(right.run.round) ||
    (number(left.run.row.order, "run order") - number(right.run.row.order, "run order")) ||
    text(left.run.record.trialId, "trial id").localeCompare(text(right.run.record.trialId, "trial id")) ||
    left.run.signalIndex - right.run.signalIndex;
}

function rowOf(id: string, group: Group): WorksheetRow {
  const lines = new Set(group.occurrences.map((one) => one.candidate.line).filter((one): one is number => one !== null));
  return {
    worksheetId: id,
    signalCategory: group.candidate.signalCategory,
    gate: group.candidate.gate,
    taskIntent: group.candidate.taskIntent,
    site: {
      file: group.candidate.file,
      line: lines.size === 1 ? [...lines][0] : null,
      text: group.candidate.text,
    },
    values: group.candidate.values,
    context: group.candidate.context,
    remedy: group.candidate.remedy,
  };
}

function occurrence(one: Occurrence): RawRecord {
  const record = one.run.record;
  return {
    round: one.run.round,
    trialId: record.trialId,
    family: record.family,
    variant: record.variant,
    arm: record.arm,
    order: record.order,
    repetition: record.repetition,
    replaces: record.replaces ?? null,
    taskId: record.taskId,
    promptSha256: object(record.fixture, "fixture").promptSha256,
    gate: record.gate,
    signal: one.run.signal,
  };
}

function quote(value: string): string {
  return value.split("\n").map((line) => line === "" ? ">" : "> " + line).join("\n");
}

function markdown(rows: WorksheetRow[], counts: Counts): string {
  const out = [
    "# Blinded signal worksheet",
    "",
    "This worksheet is prepared for human labeling. It contains no arm, round, repetition, trial, delivery, or post-signal outcome fields.",
    "Label each row exactly one of `valid-regression`, `valid-review`, or `undesired`.",
    "",
    "- Included records: " + String(counts.includedRecords),
    "- Signal occurrences: " + String(counts.signalOccurrences),
    "- Distinct worksheet rows: " + String(counts.worksheetRows),
    "",
  ];
  for (const row of rows) {
    out.push(
      "## " + row.worksheetId,
      "",
      "- Signal category: " + row.signalCategory,
      "- Gate: " + row.gate,
      "- Site: `" + row.site.file + (row.site.line === null ? "" : ":" + String(row.site.line)) + "`" + (row.site.text ? " — `" + row.site.text + "`" : ""),
      "- Measured values: `" + stable(row.values) + "`",
      "",
      "### Task intent",
      "",
      quote(row.taskIntent),
      "",
      "### Signal-time context",
      "",
      "Frozen base tree excerpt from `" + row.context.path + "`:",
      "",
      "```text",
      row.context.excerpt,
      "```",
      ...(row.context.note ? ["", "_" + row.context.note + "_"] : []),
      "",
      "### Proposed remedy or review question",
      "",
      row.remedy ? quote(row.remedy) : "(none recorded)",
      "",
    );
  }
  return out.join("\n");
}

function write(into: string, rows: WorksheetRow[], counts: Counts, joins: { worksheetId: string; occurrences: RawRecord[] }[], sets: LoadedSet[]): void {
  fs.mkdirSync(into, { recursive: true });
  const worksheet = { version: 1, status: "human-labeling-gate", counts, rows };
  const joined = {
    version: 1,
    status: "sealed-before-human-labeling",
    counts,
    evidence: sets.map((one) => ({
      round: one.input.name,
      setId: one.setId,
      manifestSha256: one.manifestSha256,
      archiveSha256: one.archiveSha256,
      archiveBytes: one.archiveBytes,
      includedRecords: one.selectedRecords.length,
      signalOccurrences: one.runs.length,
    })),
    rows: joins,
  };
  fs.writeFileSync(path.join(into, "worksheet.json"), JSON.stringify(worksheet, null, 2) + "\n");
  fs.writeFileSync(path.join(into, "worksheet.md"), markdown(rows, counts));
  fs.writeFileSync(path.join(into, "join.sealed.json"), JSON.stringify(joined, null, 2) + "\n");
  fs.writeFileSync(
    path.join(into, "README.md"),
    "# Blinded benchmark worksheet\n\nThe worksheet is ready for human labeling. It contains " +
      String(counts.includedRecords) +
      " included records, " +
      String(counts.signalOccurrences) +
      " signal occurrences and " +
      String(counts.worksheetRows) +
      " distinct rows. Label each row exactly one of `valid-regression`, `valid-review`, or `undesired`.\n\nDo not unblind or edit `join.sealed.json` before the labels are canonicalized and hashed. This preparation intentionally stops before labels, synthesis, and issue filing.\n",
  );
}

export function prepare(inputs: EvidenceInput[], into: string): Preparation {
  if (inputs.length !== 2 || new Set(inputs.map((one) => one.name)).size !== 2 || !inputs.some((one) => one.name === "v1") || !inputs.some((one) => one.name === "v2")) {
    fail("worksheet preparation needs exactly one v1 and one v2 evidence set");
  }
  const sets: LoadedSet[] = [];
  try {
    for (const input of inputs) sets.push(load(input));
    const groups = new Map<string, Group>();
    for (const set of sets) {
      for (const run of set.runs) {
        const one = candidate(run, run.signal, run.context);
        const held = groups.get(one.key);
        if (held) held.occurrences.push({ candidate: one, run });
        else groups.set(one.key, { candidate: one, occurrences: [{ candidate: one, run }] });
      }
    }
    const ordered = [...groups.values()].sort((left, right) => left.candidate.key.localeCompare(right.candidate.key));
    const rows = ordered.map((group, index) => rowOf("S" + String(index + 1).padStart(3, "0"), group));
    const counts = {
      includedRecords: sets.reduce((sum, one) => sum + one.selectedRecords.length, 0),
      signalOccurrences: sets.reduce((sum, one) => sum + one.runs.length, 0),
      worksheetRows: rows.length,
    };
    const joins = ordered.map((group, index) => ({
      worksheetId: "S" + String(index + 1).padStart(3, "0"),
      occurrences: group.occurrences.sort(occurrenceOrder).map((one) => occurrence(one)),
    }));
    write(into, rows, counts, joins, sets);
    return { counts, rows };
  } finally {
    for (const set of sets) fs.rmSync(set.root, { recursive: true, force: true });
  }
}
