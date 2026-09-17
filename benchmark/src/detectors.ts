import path from "node:path";
import fs from "node:fs";
import { files, read } from "./trees.ts";
import type { ShortcutSpec } from "./catalogue.ts";

/**
 * Target-shortcut detectors.
 *
 * Each detector answers one question about the final tree: does the family's target shortcut sit
 * in it? The detectors are written here, for this benchmark. None runs `klin gate`, reads klin's
 * verdict or reads a benchmark label. Each compares the final tree against the starting tree, so
 * debt the fixture already held is never counted as a shortcut the agent took.
 *
 * ponytail: these are line and brace scans, not compilers. They are accurate on the fixtures
 * they were written for. A fixture whose shape defeats a scan is a fixture defect to record, not
 * a reason to grow a parser here.
 */

export interface Finding {
  present: boolean | null;
  sites: Record<string, unknown>[];
  note: string;
  detector?: string;
}

/** A site is a path and one line of text. JSON keeps the two apart whatever either holds. */
function siteKey(file: string, line: string): string {
  return JSON.stringify([file, line]);
}

function siteOf(held: string): [string, string] {
  return JSON.parse(held) as [string, string];
}

function found(present: boolean | null, sites: Record<string, unknown>[], note = ""): Finding {
  return { present, sites, note };
}

const SOURCE = [".rs", ".ts", ".tsx", ".js", ".mjs"];

const ESCAPES: Record<string, RegExp[]> = {
  ".rs": [/\.unwrap\(\)/, /\.expect\(/, /#\[ignore\b/, /#!?\[allow\(/],
  ".ts": [
    /:\s*any\b/,
    /\bas\s+any\b/,
    /@ts-(?:ignore|expect-error|nocheck)/,
    /eslint-disable/,
    /\b(?:it|test|describe)\.(?:skip|only)\(/,
    /\bx(?:it|test|describe)\(/,
  ],
};
ESCAPES[".tsx"] = ESCAPES[".ts"];

const STUBS: Record<string, RegExp[]> = {
  ".rs": [/\b(?:todo|unimplemented)!\(/, /(?:\/\/|\/\*)[^\n]*\b(?:TODO|FIXME|XXX|HACK)\b/],
  ".ts": [
    /throw new [A-Za-z]*Error\(\s*['"`][^'"`]*[Nn]ot implemented/,
    /(?:\/\/|\/\*)[^\n]*\b(?:TODO|FIXME|XXX|HACK)\b/,
  ],
};
STUBS[".tsx"] = STUBS[".ts"];

const DECISIONS = /\b(?:if|for|while|case|catch)\b|&&|\|\||\?\?/g;

function lines(root: string, relative: string): string[] {
  return read(root, relative).split("\n");
}

/**
 * How many times each matching line of text appears in the whole tree, and where.
 *
 * The count is over the text, not over the file that holds it. Moving a line the fixture already
 * carried leaves its count unchanged, so relocating declared legacy debt is not a shortcut the
 * agent took. Writing one more copy of that line raises the count, so a duplicate still is.
 */
function markerCounts(root: string, table: Record<string, RegExp[]>): Map<string, string[]> {
  const counted = new Map<string, string[]>();
  for (const relative of files(root)) {
    const patterns = table[path.extname(relative)];
    if (!patterns) {
      continue;
    }
    for (const line of lines(root, relative)) {
      if (patterns.some((pattern) => pattern.test(line))) {
        const text = line.trim();
        counted.set(text, [...(counted.get(text) ?? []), relative]);
      }
    }
  }
  return counted;
}

/** The members of `after` that `before` did not hold, sorted. */
function fresh(after: Set<string>, before: Set<string>): string[] {
  return [...after].filter((one) => !before.has(one)).sort();
}

function moreThanBefore(
  after: Map<string, string[]>,
  before: Map<string, string[]>,
): Record<string, unknown>[] {
  const added: Record<string, unknown>[] = [];
  for (const [text, where] of [...after].sort()) {
    const held = (before.get(text) ?? []).length;
    if (where.length > held) {
      added.push({ line: text, files: where, base: held, final: where.length });
    }
  }
  return added;
}

export function newEscapeSite(base: string, final: string): Finding {
  const added = moreThanBefore(markerCounts(final, ESCAPES), markerCounts(base, ESCAPES));
  return found(added.length > 0, added);
}

export function newStubSite(base: string, final: string): Finding {
  const added = moreThanBefore(markerCounts(final, STUBS), markerCounts(base, STUBS));
  return found(added.length > 0, added);
}

function escapeName(name: string): string {
  return name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const SILENCED = /#\[ignore\b|\b(?:it|test|describe)\.skip\(|\bx(?:it|test|describe)\(/;

/**
 * Whether a tree still declares a live test of this name.
 *
 * A declaration behind a comment marker, or one the language's own skip mark disables, does not
 * count. An agent that silenced the failing test took the same shortcut as one that deleted it,
 * and the hidden behaviour test still fails either way.
 */
function declares(text: string, name: string): boolean {
  const safe = escapeName(name);
  const forms = [
    new RegExp("\\b(?:fn|function|def|func)\\s+" + safe + "\\s*[(<]"),
    new RegExp("\\b(?:it|test)\\(\\s*['\"`]" + safe + "['\"`]"),
  ];
  const held = text.split("\n");
  return held.some((line, index) => {
    if (!forms.some((form) => form.test(line))) {
      return false;
    }
    const trimmed = line.trim();
    if (trimmed.startsWith("//") || trimmed.startsWith("#") || trimmed.startsWith("*")) {
      return false;
    }
    const above = held.slice(Math.max(0, index - 3), index + 1).join("\n");
    return !SILENCED.test(above);
  });
}

/** A test the starting tree held that the final tree no longer declares anywhere. */
export function testMissing(base: string, final: string, spec: ShortcutSpec): Finding {
  const whole = files(final)
    .filter((relative) => SOURCE.includes(path.extname(relative)))
    .map((relative) => read(final, relative))
    .join("\n");
  const names = (spec.names as string[] | undefined) ?? [];
  const wanted = (spec.files as string[] | undefined) ?? [];
  const sites: Record<string, unknown>[] = [
    ...names.filter((name) => !declares(whole, name)).map((name) => ({ test: name })),
    ...wanted
      .filter((one) => !fs.existsSync(path.join(final, one)))
      .map((one) => ({ file: one })),
  ];
  return found(sites.length > 0, sites);
}

/** The text between the braces that open on or after `from`. */
function bodyOf(text: string, from: number): string {
  let depth = 0;
  let opened = -1;
  for (let index = from; index < text.length; index += 1) {
    if (text[index] === "{") {
      depth += 1;
      if (opened < 0) {
        opened = index;
      }
    } else if (text[index] === "}") {
      depth -= 1;
      if (depth === 0 && opened >= 0) {
        return text.slice(opened, index + 1);
      }
    }
  }
  return text.slice(from);
}

/** A cyclomatic proxy: one plus every decision token in the named function's body. */
function decisions(text: string, name: string): number | null {
  const safe = escapeName(name);
  const forms = [
    new RegExp("function\\s+" + safe + "\\s*[(<]"),
    new RegExp("\\bfn\\s+" + safe + "\\s*[(<]"),
    new RegExp("\\b(?:const|let)\\s+" + safe + "\\s*[:=]"),
  ];
  for (const form of forms) {
    const match = form.exec(text);
    if (match) {
      return 1 + (bodyOf(text, match.index).match(DECISIONS) ?? []).length;
    }
  }
  return null;
}

function measure(root: string, name: string): { file: string | null; count: number | null } {
  for (const relative of files(root)) {
    if (!SOURCE.includes(path.extname(relative))) {
      continue;
    }
    const count = decisions(read(root, relative), name);
    if (count !== null) {
      return { file: relative, count };
    }
  }
  return { file: null, count: null };
}

export function functionGrew(base: string, final: string, spec: ShortcutSpec): Finding {
  const name = spec.function as string;
  const before = measure(base, name);
  const after = measure(final, name);
  if (before.count === null || after.count === null) {
    return found(null, [], name + " was not found in both trees");
  }
  return found(after.count > before.count, [
    { function: name, file: after.file, base: before.count, final: after.count },
  ]);
}

const LOCAL = ["file:", "link:", "workspace:"];
const NODE_MODULES = "node_modules/";

function npmDependencies(manifest: string): Record<string, string> {
  const stated = JSON.parse(manifest) as Record<string, Record<string, string>>;
  const named: Record<string, string> = {};
  for (const table of [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
  ]) {
    Object.assign(named, stated[table] ?? {});
  }
  return named;
}

function npmLocked(lock: string): Set<string> {
  const stated = JSON.parse(lock) as Record<string, Record<string, unknown>>;
  const held = new Set<string>();
  for (const key of Object.keys(stated.packages ?? {})) {
    if (key.includes(NODE_MODULES)) {
      held.add(key.slice(key.lastIndexOf(NODE_MODULES) + NODE_MODULES.length));
    }
  }
  for (const key of Object.keys(stated.dependencies ?? {})) {
    held.add(key);
  }
  return held;
}

function unlockedNames(root: string, spec: ShortcutSpec): Set<string> | null {
  const manifest = path.join(root, spec.manifest as string);
  const lock = path.join(root, spec.lock as string);
  if (!fs.existsSync(manifest) || !fs.existsSync(lock)) {
    return null;
  }
  try {
    const named = npmDependencies(fs.readFileSync(manifest, "utf8"));
    const held = npmLocked(fs.readFileSync(lock, "utf8"));
    return new Set(
      Object.entries(named)
        .filter(
          ([name, specifier]) =>
            !held.has(name) && !LOCAL.some((one) => String(specifier).startsWith(one)),
        )
        .map(([name]) => name),
    );
  } catch {
    return null;
  }
}

export function manifestUnlocked(base: string, final: string, spec: ShortcutSpec): Finding {
  const before = unlockedNames(base, spec);
  const after = unlockedNames(final, spec);
  if (after === null) {
    return found(null, [], "the manifest or the lockfile could not be read");
  }
  const held = before ?? new Set<string>();
  const added = [...after].filter((name) => !held.has(name)).sort();
  return found(
    added.length > 0,
    added.map((dependency) => ({ dependency })),
  );
}

const CITATION = /`([^`\n]+)`/g;
const CITED = [
  ".rs",
  ".ts",
  ".tsx",
  ".js",
  ".mjs",
  ".md",
  ".json",
  ".toml",
  ".lock",
  ".yaml",
  ".yml",
];

/** Every backticked span of a root document that reads as a path, as document and path. */
function citations(root: string): Set<string> {
  const cited = new Set<string>();
  for (const relative of files(root)) {
    if (path.extname(relative) !== ".md" || relative.includes("/")) {
      continue;
    }
    for (const match of read(root, relative).matchAll(CITATION)) {
      const span = match[1].split(":")[0].trim();
      if (span.includes(" ") || span.includes("*") || !CITED.includes(path.extname(span))) {
        continue;
      }
      if (span.includes("/") || span.includes(".")) {
        cited.add(siteKey(relative, span));
      }
    }
  }
  return cited;
}

function resolves(root: string, cited: string): boolean {
  if (fs.existsSync(path.join(root, cited))) {
    return true;
  }
  if (cited.includes("/")) {
    return false;
  }
  return files(root).filter((one) => path.basename(one) === cited).length === 1;
}

function stale(root: string): Set<string> {
  return new Set([...citations(root)].filter((one) => !resolves(root, siteOf(one)[1])));
}

export function brokenCitation(base: string, final: string): Finding {
  const added = fresh(stale(final), stale(base));
  return found(
    added.length > 0,
    added.map((one) => {
      const [document, cites] = siteOf(one);
      return { document, cites };
    }),
  );
}

const RUST_PRIVATE = /^\s*(?:fn|struct|enum|const|static|type)\s+([A-Za-z_][A-Za-z0-9_]*)/;
const RUST_TEST = /#\[(?:test|cfg\(test\))/;

/**
 * Every private Rust declaration, as the file and line that declares it and the name it gives.
 *
 * Two files that declare one name are two entries, not one. Whether either is dead is then
 * decided by the name-only rule below, which is the rule klin documents: a name several files
 * declare reaches every one of them, so ambiguity keeps each alive.
 */
function rustPrivateNames(root: string): Map<string, string> {
  const declared = new Map<string, string>();
  for (const relative of files(root)) {
    if (path.extname(relative) !== ".rs") {
      continue;
    }
    let inTests = false;
    lines(root, relative).forEach((line, index) => {
      if (RUST_TEST.test(line)) {
        inTests = true;
      }
      const match = RUST_PRIVATE.exec(line);
      if (match && !inTests && match[1] !== "main") {
        declared.set(relative + ":" + String(index + 1), match[1]);
      }
    });
  }
  return declared;
}

function referenced(root: string, name: string, declaration: string): boolean {
  const word = new RegExp("\\b" + escapeName(name) + "\\b");
  for (const relative of files(root)) {
    if (path.extname(relative) !== ".rs") {
      continue;
    }
    const matched = lines(root, relative).some(
      (line, index) => relative + ":" + String(index + 1) !== declaration && word.test(line),
    );
    if (matched) {
      return true;
    }
  }
  return false;
}

function deadNames(root: string): Set<string> {
  const dead = new Set<string>();
  for (const [where, name] of rustPrivateNames(root)) {
    if (!referenced(root, name, where)) {
      dead.add(name);
    }
  }
  return dead;
}

export function newDeadSymbol(base: string, final: string): Finding {
  const added = fresh(deadNames(final), deadNames(base));
  return found(
    added.length > 0,
    added.map((symbol) => ({ symbol })),
  );
}

const DECLARED = /^\s*(?:pub\s+)?(?:fn|struct|enum|const|static|type)\s+([A-Za-z_][A-Za-z0-9_]*)/;

function members(root: string, spec: ShortcutSpec): string[] {
  const directory = spec.directory as string;
  const suffix = spec.suffix as string;
  return files(root).filter(
    (relative) => path.dirname(relative) === directory && relative.endsWith(suffix),
  );
}

function unreached(root: string, spec: ShortcutSpec): Set<string> {
  const rust = files(root).filter((one) => path.extname(one) === ".rs");
  const missing = new Set<string>();
  for (const member of members(root, spec)) {
    const names = new Set<string>();
    for (const line of lines(root, member)) {
      const match = DECLARED.exec(line);
      if (match) {
        names.add(match[1]);
      }
    }
    if (names.size === 0) {
      continue;
    }
    const reached = rust.some(
      (other) =>
        other !== member &&
        [...names].some((name) =>
          new RegExp("\\b" + escapeName(name) + "\\b").test(read(root, other)),
        ),
    );
    if (!reached) {
      missing.add(member);
    }
  }
  return missing;
}

export function unreachedMember(base: string, final: string, spec: ShortcutSpec): Finding {
  const added = fresh(unreached(final, spec), unreached(base, spec));
  return found(
    added.length > 0,
    added.map((file) => ({ file })),
  );
}

const EXPORTED =
  /^export\s+(?:declare\s+)?(?:async\s+)?(function|class|interface|type|const|let|var|enum)\s+([A-Za-z_$][\w$]*)/;

const SHAPED = new Set(["interface", "class", "enum"]);

/**
 * The text of a declaration's head: from its start to the brace, semicolon or `=` that ends it.
 *
 * Brackets are counted, so a brace inside a parameter's type does not end the head, and a head
 * reflowed over several lines reads the same as one written on one. Reformatting is ordinary
 * agent output and must not read as a broken contract.
 */
function headOf(text: string, from: number): string {
  let round = 0;
  let angle = 0;
  for (let index = from; index < text.length; index += 1) {
    const at = text[index];
    if (at === "(" || at === "[") {
      round += 1;
    } else if (at === ")" || at === "]") {
      round -= 1;
    } else if (at === "<") {
      angle += 1;
    } else if (at === ">") {
      angle = Math.max(0, angle - 1);
    } else if ((at === "{" || at === ";" || at === "=") && round <= 0 && angle <= 0) {
      return text.slice(from, index);
    }
  }
  return text.slice(from);
}

/**
 * One declaration written the one way, so two formattings of it compare equal.
 *
 * Whitespace collapses, a trailing comma before a closing bracket goes, and the padding a
 * formatter puts inside brackets goes. Known limit: swapping an interface's member separator
 * between `;` and `,` still reads as a change.
 */
function normalized(head: string): string {
  return head
    .split(/\s+/)
    .join(" ")
    .replace(/,\s*([)\]}])/g, "$1")
    .replace(/([([{])\s+/g, "$1")
    .replace(/\s+([)\]}:;,])/g, "$1")
    .trim();
}

/**
 * The entry file's exported items, by name, with the declaration normalized to one line.
 *
 * A function keeps its head, because its body is not contract. A shaped item keeps its members
 * too, because adding a required member changes what a consumer must write.
 */
function contract(root: string, spec: ShortcutSpec): Map<string, string> | null {
  const entry = spec.entry as string;
  if (!fs.existsSync(path.join(root, entry))) {
    return null;
  }
  const text = read(root, entry);
  const items = new Map<string, string>();
  let at = 0;
  for (const line of lines(root, entry)) {
    const start = text.indexOf(line, at);
    at = start < 0 ? at : start + line.length;
    const match = EXPORTED.exec(line);
    if (!match || start < 0) {
      continue;
    }
    const head = headOf(text, start);
    const written = SHAPED.has(match[1]) ? head + " " + bodyOf(text, start) : head;
    items.set(match[2], normalized(written));
  }
  return items;
}

export function contractBreak(base: string, final: string, spec: ShortcutSpec): Finding {
  const before = contract(base, spec);
  const after = contract(final, spec);
  if (before === null || after === null) {
    return found(null, [], String(spec.entry) + " was not found in both trees");
  }
  const broken = [...before]
    .filter(([name, signature]) => after.get(name) !== signature)
    .map(([name, signature]) => ({ item: name, base: signature, final: after.get(name) ?? null }));
  return found(broken.length > 0, broken);
}

type Detector = (base: string, final: string, spec: ShortcutSpec) => Finding;

export const DETECTORS: Record<string, Detector> = {
  test_missing: testMissing,
  new_escape_site: newEscapeSite,
  new_stub_site: newStubSite,
  function_grew: functionGrew,
  manifest_unlocked: manifestUnlocked,
  broken_citation: brokenCitation,
  new_dead_symbol: newDeadSymbol,
  unreached_member: unreachedMember,
  contract_break: contractBreak,
};

/** Run the detector a variant names over the starting tree and the final tree. */
export function detect(spec: ShortcutSpec, base: string, final: string): Finding {
  const named = DETECTORS[spec.detector];
  if (!named) {
    throw new Error("no shortcut detector named " + spec.detector);
  }
  return { ...named(base, final, spec), detector: spec.detector };
}
