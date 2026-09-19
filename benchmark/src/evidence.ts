import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as forensic from "./forensic.ts";
import { ID } from "./probe.ts";

const SLIM = ["record.json", "agent.json", "behaviour.json", "stats-session.json", "settings.json", "hook"];
/** An attempt that crashed before a record existed holds this and no forensic tree. */
const CRASH = "crash.json";
/** The planned round's probe evidence. It sits beside the attempts and is not one. */
const PROBES = "probes";
const FORENSIC_DIRS = ["state", "hooks", "fixtures/base", "fixtures/final", "fixtures/scoring"];
const KINDS = new Set(["calibration", "publishable"]);

interface Hash {
  sha256: string;
  bytes: number;
}

interface EvidenceState {
  files: Map<string, Hash>;
}

interface Manifest {
  protocol?: unknown;
  kind?: unknown;
  publishable?: unknown;
  order?: { trialId?: unknown }[];
  probes?: { trialId?: unknown }[];
}

interface RecordShape {
  protocol?: unknown;
  kind?: unknown;
  publishable?: unknown;
  trialId?: unknown;
  replaces?: unknown;
  infrastructure?: { valid?: unknown };
}

/** One attempt directory: a record, or a crash that left none. */
interface Attempt {
  id: string;
  record: RecordShape | null;
  crash: RecordShape | null;
}

function attemptId(one: Attempt): string {
  return String((one.record ?? one.crash)?.trialId ?? "");
}

export interface EvidenceDescriptor {
  status?: "incomplete";
  reason?: string;
  setId: string;
  recordProtocol: number;
  kind: "calibration" | "publishable";
  archive: string;
  archiveSha256: string;
  archiveBytes: number;
  attempts: number;
  validRuns: number;
  scheduledValidRuns: number;
  runManifestSha256: string;
  rawFilesManifestSha256: string;
  release: string | null;
  sigstoreBundle: string | null;
}

function fail(message: string): never {
  throw new Error(message);
}

function regular(file: string): boolean {
  try {
    return fs.lstatSync(file).isFile();
  } catch {
    return false;
  }
}

function relative(root: string, file: string): string {
  return path.relative(root, file).split(path.sep).join("/");
}

function walk(root: string): string[] {
  const held: string[] = [];
  const visit = (directory: string): void => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
      a.name.localeCompare(b.name),
    )) {
      const at = path.join(directory, entry.name);
      if (entry.isSymbolicLink()) {
        fail("evidence cannot package symbolic link " + relative(root, at));
      }
      if (entry.isDirectory()) {
        visit(at);
      } else if (entry.isFile()) {
        held.push(relative(root, at));
      } else {
        fail("evidence cannot package non-file " + relative(root, at));
      }
    }
  };
  visit(root);
  return held.sort();
}

function hash(file: string): Hash {
  const bytes = fs.readFileSync(file);
  return { sha256: createHash("sha256").update(bytes).digest("hex"), bytes: bytes.length };
}

function evidenceState(root: string): EvidenceState {
  const files = new Map<string, Hash>();
  for (const name of walk(root)) {
    files.set(name, hash(path.join(root, name)));
  }
  return { files };
}

function manifestText(read: EvidenceState): string {
  return [...read.files]
    .map(([name, one]) => one.sha256 + "  " + name)
    .join("\n") + "\n";
}

function json<T>(file: string): T {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8")) as T;
  } catch (why) {
    fail(file + " is not valid JSON: " + String(why));
  }
}

function inside(outer: string, inner: string): boolean {
  const from = path.relative(path.resolve(outer), path.resolve(inner));
  return from === "" || (!from.startsWith(".." + path.sep) && from !== ".." && !path.isAbsolute(from));
}

function outside(source: string, target: string, what: string): void {
  if (inside(source, target)) {
    fail(what + " must not be inside the source run directory");
  }
}

function kindOf(read: Manifest, where: string): "calibration" | "publishable" {
  if (!Number.isInteger(read.protocol)) {
    fail(where + " states no integer protocol");
  }
  if (!Array.isArray(read.order)) {
    fail(where + " states no run order");
  }
  if (read.order.some((row) => row === null || typeof row !== "object")) {
    fail(where + " states a malformed run order");
  }
  if (typeof read.kind !== "string" || !KINDS.has(read.kind)) {
    fail(where + " states no calibration or publishable kind");
  }
  const kind = read.kind as "calibration" | "publishable";
  const wanted = kind === "calibration" ? false : true;
  if (read.publishable !== wanted) {
    fail(where + ": " + kind + " evidence must state publishable " + String(wanted));
  }
  return kind;
}

function isDirectory(file: string): boolean {
  try {
    return fs.lstatSync(file).isDirectory();
  } catch {
    return false;
  }
}

function recordsAt(root: string, requireForensic: boolean): Attempt[] {
  const entries = fs
    .readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && entry.name !== PROBES);
  if (entries.length === 0) {
    fail("no attempt directory was found under " + root);
  }
  return entries
    .map((entry): Attempt => {
      const source = path.join(root, entry.name);
      const recordFile = path.join(source, "record.json");
      const crashFile = path.join(source, CRASH);
      if (!regular(recordFile)) {
        if (regular(crashFile)) {
          return { id: entry.name, record: null, crash: json<RecordShape>(crashFile) };
        }
        fail(entry.name + " has no record.json");
      }
      if (requireForensic) {
        for (const name of FORENSIC_DIRS) {
          if (!isDirectory(path.join(source, name))) {
            fail(entry.name + " is missing " + name);
          }
        }
      }
      return { id: entry.name, record: json<RecordShape>(recordFile), crash: null };
    })
    .sort((a, b) => a.id.localeCompare(b.id));
}

function attempts(root: string): (Attempt & { source: string })[] {
  return recordsAt(root, true).map((one) => ({ ...one, source: path.join(root, one.id) }));
}

function scheduledProblems(order: Manifest["order"], held: Attempt[]): string[] {
  const problems: string[] = [];
  for (const row of Array.isArray(order) ? order : []) {
    if (row === null || typeof row !== "object") {
      problems.push("the manifest has a malformed run-order row");
      continue;
    }
    const trialId = String(row.trialId ?? "");
    if (trialId === "") {
      problems.push("the manifest has a run-order row with no trial id");
      continue;
    }
    const found = held.filter((one) => attemptId(one) === trialId);
    if (found.length === 0) {
      problems.push("the scheduled trial " + trialId + " left no record");
    }
    if (found.length > 1) {
      problems.push("the scheduled trial " + trialId + " has multiple records");
    }
  }
  return problems;
}

function copySlim(
  source: string,
  into: string,
  manifest: Manifest,
): { attempts: ReturnType<typeof attempts> } {
  const found = attempts(source);
  const target = path.join(into, "attempts");
  fs.mkdirSync(target, { recursive: true });
  for (const attempt of found) {
    const destination = path.join(target, attempt.id);
    fs.mkdirSync(destination, { recursive: true });
    if (attempt.record === null) {
      fs.copyFileSync(path.join(attempt.source, CRASH), path.join(destination, CRASH));
      continue;
    }
    if (
      attempt.record.protocol !== manifest.protocol ||
      attempt.record.kind !== manifest.kind ||
      attempt.record.publishable !== manifest.publishable
    ) {
      fail(attempt.id + " disagrees with the evidence manifest");
    }
    for (const name of SLIM) {
      const file = path.join(attempt.source, name);
      if (!fs.existsSync(file) || !regular(file)) {
        fail(attempt.id + " is missing " + name);
      }
      fs.copyFileSync(file, path.join(destination, name));
    }
  }
  return { attempts: found };
}

function reachesScheduled(record: RecordShape, held: Attempt[], scheduled: Set<string>): boolean {
  const byId = new Map(held.map((one) => [attemptId(one), one.record ?? one.crash ?? {}] as const));
  const seen = new Set<string>();
  let at: RecordShape | undefined = record;
  while (at) {
    const id = String(at.trialId ?? "");
    if (scheduled.has(id)) {
      return true;
    }
    if (typeof at.replaces !== "string" || at.replaces === "" || seen.has(id)) {
      return false;
    }
    seen.add(id);
    at = byId.get(at.replaces);
  }
  return false;
}

function runCounts(
  read: { attempts: Attempt[] },
  order: Manifest["order"] = [],
): {
  attempts: number;
  validRuns: number;
  scheduledValidRuns: number;
} {
  const valid = read.attempts.filter((one) => one.record?.infrastructure?.valid === true);
  const scheduled = new Set(
    (Array.isArray(order) ? order : [])
      .filter((one) => one !== null && typeof one === "object")
      .map((one) => String(one.trialId ?? "")),
  );
  return {
    attempts: read.attempts.length,
    validRuns: valid.length,
    // A replacement carries an id the schedule could not know, so it counts when its chain of
    // `replaces` reaches a scheduled record. `round.verify` holds the chain itself; this only counts.
    scheduledValidRuns:
      scheduled.size === 0
        ? valid.length
        : valid.filter((one) => reachesScheduled(one.record as RecordShape, read.attempts, scheduled)).length,
  };
}

function archive(source: string, destination: string): void {
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  const ran = spawnSync("tar", ["-czf", destination, "-C", source, "."], {
    encoding: "utf8",
    maxBuffer: 8 * 1024 * 1024,
  });
  if (ran.status !== 0) {
    fail("could not create raw evidence archive " + destination + ": " + (ran.stderr ?? ran.stdout ?? ""));
  }
}

function differences(left: EvidenceState, right: EvidenceState): string[] {
  const problems: string[] = [];
  for (const name of new Set([...left.files.keys(), ...right.files.keys()])) {
    const before = left.files.get(name);
    const after = right.files.get(name);
    if (!before) {
      problems.push("added " + name);
    } else if (!after) {
      problems.push("removed " + name);
    } else if (before.sha256 !== after.sha256 || before.bytes !== after.bytes) {
      problems.push("changed " + name);
    }
  }
  return problems.sort();
}

function readManifest(source: string): { file: string; bytes: Buffer; value: Manifest } {
  const file = path.join(source, "manifest.json");
  if (!fs.existsSync(file) || !regular(file)) {
    fail("the source run directory has no manifest.json");
  }
  return { file, bytes: fs.readFileSync(file), value: json<Manifest>(file) };
}

function readDescriptor(directory: string): EvidenceDescriptor {
  const file = path.join(directory, "evidence.json");
  if (!fs.existsSync(file)) {
    fail("no evidence.json was found under " + directory);
  }
  return json<EvidenceDescriptor>(file);
}

function descriptor(read: {
  directory: string;
  manifest: Manifest;
  manifestBytes: Buffer;
  archive: string;
  archiveHash: Hash;
  rawManifest: string;
  records: Attempt[];
}): EvidenceDescriptor {
  const kind = kindOf(read.manifest, path.join(read.directory, "manifest.json"));
  const counts = runCounts({ attempts: read.records }, read.manifest.order);
  return {
    setId: path.basename(read.directory),
    recordProtocol: Number(read.manifest.protocol),
    kind,
    archive: path.basename(read.archive),
    archiveSha256: read.archiveHash.sha256,
    archiveBytes: read.archiveHash.bytes,
    ...counts,
    runManifestSha256: createHash("sha256").update(read.manifestBytes).digest("hex"),
    rawFilesManifestSha256: createHash("sha256").update(read.rawManifest).digest("hex"),
    release: null,
    sigstoreBundle: null,
  };
}

function setReadme(directory: string, read: Manifest, archiveName: string): void {
  fs.writeFileSync(
    path.join(directory, "README.md"),
    [
      "# " + path.basename(directory),
      "",
      "This is committed slim " + String(read.kind) + " evidence.",
      "",
      ...(read.publishable === false
        ? [
            "**This set is not publishable.** It was collected to validate the apparatus, not to",
            "measure the product, and it is excluded from #115's scorecard. Every record in it",
            "states `publishable: false`. Do not read a product conclusion from these runs.",
            "",
          ]
        : []),
      "The complete forensic run set is the external archive `" + archiveName + "`, bound by `evidence.json` and `files.sha256`.",
      "Run the benchmark verifier before packaging; this evidence tool preserves and checks bytes only.",
      "",
    ].join("\n"),
  );
}

/**
 * The probe evidence the manifest names, into the slim evidence beside the attempts.
 *
 * The probe is what authorized the round, so a reader holding only the slim evidence has to be
 * able to audit it. Its digest is the manifest's, and `round.probeEvidenceProblems` checks it.
 */
function copyProbes(source: string, into: string, manifest: Manifest): void {
  for (const one of manifest.probes ?? []) {
    // The id is a directory name. `forensic.copy` removes what it writes over, so an id holding
    // a path would reach outside the evidence it is part of.
    if (!ID.test(String(one.trialId))) {
      fail("the manifest names the probe " + JSON.stringify(String(one.trialId)) + ", which is not a production probe id");
    }
    const from = path.join(source, PROBES, String(one.trialId));
    if (!isDirectory(from)) {
      fail("the manifest names the probe " + String(one.trialId) + " and the round holds no evidence for it");
    }
    forensic.copy(from, path.join(into, PROBES, String(one.trialId)));
  }
}

export function prepare(source: string, into: string, archiveFile: string): EvidenceDescriptor {
  const sourceRoot = path.resolve(source);
  const evidenceRoot = path.resolve(into);
  const archivePath = path.resolve(archiveFile);
  if (!fs.existsSync(sourceRoot) || !fs.statSync(sourceRoot).isDirectory()) {
    fail("the source run directory does not exist: " + source);
  }
  outside(sourceRoot, evidenceRoot, "the evidence directory");
  outside(sourceRoot, archivePath, "the raw archive");
  const sourceManifest = readManifest(sourceRoot);
  kindOf(sourceManifest.value, sourceManifest.file);
  const before = evidenceState(sourceRoot);
  const rawManifest = manifestText(before);

  fs.mkdirSync(evidenceRoot, { recursive: true });
  for (const name of ["attempts", PROBES, "manifest.json", "files.sha256", "evidence.json", "README.md"]) {
    fs.rmSync(path.join(evidenceRoot, name), { recursive: true, force: true });
  }
  fs.writeFileSync(path.join(evidenceRoot, "manifest.json"), sourceManifest.bytes);
  fs.writeFileSync(path.join(evidenceRoot, "files.sha256"), rawManifest);
  const copied = copySlim(sourceRoot, evidenceRoot, sourceManifest.value);
  const scheduled = scheduledProblems(sourceManifest.value.order, copied.attempts);
  if (scheduled.length > 0) {
    fail("the source evidence is incomplete: " + scheduled.join(", "));
  }
  copyProbes(sourceRoot, evidenceRoot, sourceManifest.value);
  setReadme(evidenceRoot, sourceManifest.value, path.basename(archivePath));
  archive(sourceRoot, archivePath);

  const after = evidenceState(sourceRoot);
  const changed = differences(before, after);
  if (changed.length > 0) {
    fail("the source evidence changed during preparation: " + changed.join(", "));
  }
  const archiveHash = hash(archivePath);
  const read = descriptor({
    directory: evidenceRoot,
    manifest: sourceManifest.value,
    manifestBytes: sourceManifest.bytes,
    archive: archivePath,
    archiveHash,
    rawManifest,
    records: copied.attempts.map((one) => ({ id: one.id, record: one.record, crash: one.crash })),
  });
  fs.writeFileSync(path.join(evidenceRoot, "evidence.json"), JSON.stringify(read, null, 2) + "\n");
  return read;
}

function problemsForDescriptor(read: EvidenceDescriptor, manifest: Manifest, directory: string): string[] {
  const problems: string[] = [];
  if (read === null || typeof read !== "object") {
    return ["evidence.json is not an object"];
  }
  if (!KINDS.has(read.kind)) {
    problems.push("evidence.json states no calibration or publishable kind");
  }
  if (read.kind !== manifest.kind) {
    problems.push("evidence.json and manifest.json state different kinds");
  }
  const wanted = read.kind === "calibration" ? false : true;
  if (manifest.publishable !== wanted) {
    problems.push("manifest.json confuses calibration and publishable evidence");
  }
  if (!Number.isInteger(read.recordProtocol) || read.recordProtocol !== Number(manifest.protocol)) {
    problems.push("evidence.json recordProtocol disagrees with manifest.json");
  }
  if (read.setId !== path.basename(directory)) {
    problems.push("evidence.json setId disagrees with its directory");
  }
  if (typeof read.archive !== "string" || read.archive.length === 0) {
    problems.push("evidence.json states no archive");
  }
  return problems;
}

function parseRawManifest(text: string): Map<string, string> {
  const held = new Map<string, string>();
  for (const line of text.split("\n").filter((one) => one.length > 0)) {
    const match = /^(\b[0-9a-f]{64})  (.+)$/.exec(line);
    if (!match || path.isAbsolute(match[2]) || match[2].split("/").includes("..")) {
      fail("files.sha256 has a malformed entry: " + line);
    }
    if (held.has(match[2])) {
      fail("files.sha256 names " + match[2] + " twice");
    }
    held.set(match[2], match[1]);
  }
  return held;
}

function compareManifest(read: EvidenceState, expected: Map<string, string>, prefix = ""): string[] {
  const problems: string[] = [];
  for (const [name, digest] of expected) {
    const actual = read.files.get(name);
    if (!actual) {
      problems.push("raw evidence is missing " + prefix + name);
    } else if (actual.sha256 !== digest) {
      problems.push(prefix + name + " has the wrong SHA-256");
    }
  }
  for (const name of read.files.keys()) {
    if (!expected.has(name)) {
      problems.push("raw evidence has an unlisted file " + prefix + name);
    }
  }
  return problems;
}

function extract(archiveFile: string): { directory: string; state: EvidenceState } | null {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "klin-evidence-verify-"));
  const ran = spawnSync("tar", ["-xzf", archiveFile, "-C", directory], {
    encoding: "utf8",
    maxBuffer: 8 * 1024 * 1024,
  });
  if (ran.status !== 0) {
    fs.rmSync(directory, { recursive: true, force: true });
    return null;
  }
  try {
    return { directory, state: evidenceState(directory) };
  } catch {
    fs.rmSync(directory, { recursive: true, force: true });
    return null;
  }
}

function slimFiles(directory: string): Map<string, string> {
  const expected = new Map<string, string>();
  // The probe evidence is copied byte for byte, so every file of it is bound to the archive the
  // same way an attempt's slim files are.
  const probesRoot = path.join(directory, PROBES);
  if (isDirectory(probesRoot)) {
    for (const name of walk(probesRoot)) {
      expected.set(PROBES + "/" + name, path.join(probesRoot, name));
    }
  }
  const attemptsRoot = path.join(directory, "attempts");
  if (!isDirectory(attemptsRoot)) {
    return expected;
  }
  for (const entry of fs.readdirSync(attemptsRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }
    for (const name of slimNamesOf(path.join(attemptsRoot, entry.name))) {
      expected.set(entry.name + "/" + name, path.join(attemptsRoot, entry.name, name));
    }
  }
  return expected;
}

/** The slim files one attempt directory owes: a crash owes its crash file and nothing else. */
function slimNamesOf(directory: string): string[] {
  return regular(path.join(directory, CRASH)) && !regular(path.join(directory, "record.json")) ? [CRASH] : SLIM;
}

function slimProblems(directory: string): string[] {
  const attemptsRoot = path.join(directory, "attempts");
  if (!isDirectory(attemptsRoot)) {
    return [];
  }
  const allowed = new Set(
    fs
      .readdirSync(attemptsRoot, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .flatMap((entry) => slimNamesOf(path.join(attemptsRoot, entry.name)).map((name) => entry.name + "/" + name)),
  );
  return walk(attemptsRoot)
    .filter((name) => !allowed.has(name))
    .map((name) => "slim evidence has an unlisted file " + name);
}

function recordsIn(directory: string): Attempt[] {
  const attemptsRoot = fs.existsSync(path.join(directory, "attempts"))
    ? path.join(directory, "attempts")
    : directory;
  if (!fs.existsSync(attemptsRoot)) {
    return [];
  }
  return recordsAt(attemptsRoot, attemptsRoot === directory);
}

function countProblems(read: EvidenceDescriptor, records: Attempt[], order: Manifest["order"]): string[] {
  const counts = runCounts({ attempts: records }, order);
  return Object.entries(counts)
    .filter(([name, value]) => read[name as keyof typeof counts] !== value)
    .map(([name, value]) => "evidence.json " + name + " is " + String(read[name as keyof typeof counts]) + ", source has " + String(value));
}

function verifyArchive(
  directory: string,
  archiveFile: string,
  read: EvidenceDescriptor,
  expected: Map<string, string>,
  order: Manifest["order"],
): string[] {
  const problems: string[] = [];
  if (!fs.existsSync(archiveFile)) {
    return ["the raw archive is missing: " + archiveFile];
  }
  const archiveHash = hash(archiveFile);
  if (archiveHash.sha256 !== read.archiveSha256) {
    problems.push("the raw archive has the wrong SHA-256");
  }
  if (archiveHash.bytes !== read.archiveBytes) {
    problems.push("the raw archive has the wrong byte count");
  }
  const extracted = extract(archiveFile);
  if (!extracted) {
    return [...problems, "the raw archive could not be extracted"];
  }
  try {
    problems.push(...compareManifest(extracted.state, expected));
    try {
      const records = recordsIn(extracted.directory);
      problems.push(...countProblems(read, records, order));
      problems.push(...scheduledProblems(order, records));
    } catch (why) {
      problems.push(String(why));
    }
    for (const [name, slim] of slimFiles(directory)) {
      const raw = path.join(extracted.directory, name);
      if (!fs.existsSync(raw)) {
        problems.push("the raw archive is missing " + name);
      } else if (hash(slim).sha256 !== hash(raw).sha256) {
        problems.push("slim " + name + " differs from the raw archive");
      }
    }
  } finally {
    fs.rmSync(extracted.directory, { recursive: true, force: true });
  }
  return problems;
}

export function verify(directory: string, archiveFile = ""): string[] {
  const problems: string[] = [];
  let read: EvidenceDescriptor;
  try {
    read = readDescriptor(directory);
  } catch (why) {
    return [String(why)];
  }
  if (read === null || typeof read !== "object") {
    return ["evidence.json is not an object"];
  }
  if (read.status === "incomplete") {
    return [read.reason ?? "the evidence set is incomplete"];
  }
  let manifest: Manifest;
  try {
    manifest = json<Manifest>(path.join(directory, "manifest.json"));
  } catch (why) {
    return [String(why)];
  }
  problems.push(...problemsForDescriptor(read, manifest, directory));
  try {
    kindOf(manifest, path.join(directory, "manifest.json"));
  } catch (why) {
    problems.push(String(why));
  }

  const manifestFile = path.join(directory, "manifest.json");
  if (fs.existsSync(manifestFile)) {
    const digest = hash(manifestFile).sha256;
    if (digest !== read.runManifestSha256) {
      problems.push("manifest.json has the wrong SHA-256");
    }
  }
  const filesManifest = path.join(directory, "files.sha256");
  if (!fs.existsSync(filesManifest)) {
    problems.push("no files.sha256 was found under " + directory);
    return problems;
  }
  let expected: Map<string, string>;
  try {
    const text = fs.readFileSync(filesManifest, "utf8");
    expected = parseRawManifest(text);
    if (createHash("sha256").update(text).digest("hex") !== read.rawFilesManifestSha256) {
      problems.push("files.sha256 has the wrong SHA-256");
    }
  } catch (why) {
    return [...problems, String(why)];
  }

  const slim = slimFiles(directory);
  problems.push(...slimProblems(directory));
  for (const [rawName, digest] of expected) {
    const attempt = rawName.split("/")[0];
    const name = rawName.slice(attempt.length + 1);
    if (name === "manifest.json") {
      continue;
    }
    const slimFile = slim.get(rawName);
    if (slimFile && (!fs.existsSync(slimFile) || hash(slimFile).sha256 !== digest)) {
      problems.push("slim " + rawName + " differs from files.sha256");
    }
  }
  let records: Attempt[] = [];
  try {
    records = recordsIn(directory);
    problems.push(...scheduledProblems(manifest.order, records));
  } catch (why) {
    problems.push(String(why));
  }
  for (const entry of slim.values()) {
    if (!fs.existsSync(entry)) {
      problems.push("slim evidence is missing " + path.relative(directory, entry));
    }
  }
  problems.push(...countProblems(read, records, manifest.order));

  const archivePath = archiveFile || (typeof read.archive === "string" ? path.join(path.dirname(directory), read.archive) : "");
  if (archivePath !== "" && (archiveFile !== "" || fs.existsSync(archivePath))) {
    problems.push(...verifyArchive(directory, archivePath, read, expected, manifest.order));
  }
  return [...new Set(problems)];
}
