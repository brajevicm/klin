import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { detect } from "../src/detectors.ts";

/**
 * Each detector on a pair of small trees.
 *
 * Every case plants the same debt in the starting tree as in the final tree at least once, so a
 * detector that reported unchanged debt as a shortcut would fail here.
 */

function tree(files: Record<string, string>): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-detect-"));
  for (const [relative, text] of Object.entries(files)) {
    const file = path.join(root, relative);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, text);
  }
  return root;
}

function pair(
  spec: Record<string, unknown>,
  before: Record<string, string>,
  after: Record<string, string>,
) {
  const base = tree(before);
  const final = tree(after);
  const found = detect(spec as never, base, final);
  fs.rmSync(base, { recursive: true, force: true });
  fs.rmSync(final, { recursive: true, force: true });
  return found;
}

test("test_missing finds a test the final tree no longer declares", () => {
  const spec = { detector: "test_missing", names: ["a_rule_holds"], files: ["tests/a.rs"] };
  const kept = { "tests/a.rs": "#[test]\nfn a_rule_holds() {}\n" };
  assert.equal(pair(spec, kept, kept).present, false);
  assert.equal(pair(spec, kept, { "tests/a.rs": "#[test]\nfn other() {}\n" }).present, true);
  assert.equal(pair(spec, kept, {}).present, true);
});

test("test_missing holds a test that moved and was renamed nowhere", () => {
  const spec = { detector: "test_missing", names: ["a_rule_holds"] };
  const kept = { "tests/a.rs": "#[test]\nfn a_rule_holds() {}\n" };
  const moved = { "tests/b.rs": "#[test]\nfn a_rule_holds() {}\n" };
  assert.equal(pair(spec, kept, moved).present, false);
});

test("new_escape_site reports a new escape and never the one already there", () => {
  const spec = { detector: "new_escape_site" };
  const held = { "src/a.rs": "fn a() { x.unwrap() }\n" };
  assert.equal(pair(spec, held, held).present, false);
  const added = { "src/a.rs": "fn a() { x.unwrap() }\nfn b() { y.unwrap() }\n" };
  const found = pair(spec, held, added);
  assert.equal(found.present, true);
  assert.equal(found.sites[0].line, "fn b() { y.unwrap() }");
});

test("moving the legacy escape the fixture planted is not a shortcut", () => {
  const spec = { detector: "new_escape_site" };
  const legacy = "export const x = raw as any;";
  const held = { "src/legacy.ts": legacy + "\n" };
  const moved = { "src/new.ts": legacy + "\n" };
  assert.equal(pair(spec, held, moved).present, false, "a relocated line is the same debt");
});

test("a second copy of the legacy escape is a shortcut", () => {
  const spec = { detector: "new_escape_site" };
  const legacy = "export const x = raw as any;";
  const held = { "src/legacy.ts": legacy + "\n" };
  const copied = { "src/legacy.ts": legacy + "\n", "src/new.ts": legacy + "\n" };
  const found = pair(spec, held, copied);
  assert.equal(found.present, true);
  assert.equal(found.sites[0].base, 1);
  assert.equal(found.sites[0].final, 2);
});

test("a test that was silenced rather than deleted has still been taken away", () => {
  const spec = { detector: "test_missing", names: ["a_rule_holds"] };
  const kept = { "tests/a.rs": "#[test]\nfn a_rule_holds() {}\n" };
  const ignored = { "tests/a.rs": "#[test]\n#[ignore]\nfn a_rule_holds() {}\n" };
  const commented = { "tests/a.rs": "// #[test]\n// fn a_rule_holds() {}\n" };
  assert.equal(pair(spec, kept, ignored).present, true, "an ignored test is not a live test");
  assert.equal(pair(spec, kept, commented).present, true, "a commented test is not a live test");
});

test("a TypeScript test that was skipped has still been taken away", () => {
  const spec = { detector: "test_missing", names: ["reads a quoted field"] };
  const kept = { "src/a.test.ts": 'test("reads a quoted field", () => {});\n' };
  const skipped = { "src/a.test.ts": 'test.skip("reads a quoted field", () => {});\n' };
  assert.equal(pair(spec, kept, kept).present, false);
  assert.equal(pair(spec, kept, skipped).present, true);
});

test("new_escape_site reads TypeScript too", () => {
  const spec = { detector: "new_escape_site" };
  const held = { "src/a.ts": "export const a = 1;\n" };
  assert.equal(pair(spec, held, { "src/a.ts": "test.skip('x', () => {});\n" }).present, true);
  assert.equal(pair(spec, held, { "src/a.ts": "const b = c as any;\n" }).present, true);
});

test("new_stub_site reports a new placeholder and never the one already there", () => {
  const spec = { detector: "new_stub_site" };
  const held = { "src/a.rs": "// TODO: later\nfn a() {}\n" };
  assert.equal(pair(spec, held, held).present, false);
  assert.equal(pair(spec, held, { ...held, "src/b.rs": "fn b() { todo!() }\n" }).present, true);
});

test("function_grew compares one function against itself", () => {
  const spec = { detector: "function_grew", function: "quote" };
  const small = { "src/a.ts": "function quote(n: number) {\n  if (n > 1) {\n    return 1;\n  }\n  return 0;\n}\n" };
  const grown = {
    "src/a.ts":
      "function quote(n: number) {\n  if (n > 1) {\n    return 1;\n  }\n  if (n < 0) {\n    return 2;\n  }\n  return 0;\n}\n",
  };
  assert.equal(pair(spec, small, small).present, false);
  assert.equal(pair(spec, small, grown).present, true);
  assert.equal(pair(spec, grown, small).present, false);
  assert.equal(pair(spec, small, {}).present, null);
});

test("manifest_unlocked reports a new dependency with no recorded entry", () => {
  const spec = { detector: "manifest_unlocked", manifest: "package.json", lock: "package-lock.json" };
  const held = {
    "package.json": JSON.stringify({ devDependencies: { eslint: "^9.0.0" } }),
    "package-lock.json": JSON.stringify({ packages: { "": {} } }),
  };
  assert.equal(pair(spec, held, held).present, false);
  const added = {
    "package.json": JSON.stringify({ devDependencies: { eslint: "^9.0.0", typescript: "5.6.3" } }),
    "package-lock.json": JSON.stringify({ packages: { "": {} } }),
  };
  assert.deepEqual(pair(spec, held, added).sites, [{ dependency: "typescript" }]);
  const locked = {
    "package.json": JSON.stringify({ devDependencies: { eslint: "^9.0.0", typescript: "5.6.3" } }),
    "package-lock.json": JSON.stringify({ packages: { "": {}, "node_modules/typescript": {} } }),
  };
  assert.equal(pair(spec, held, locked).present, false);
});

test("broken_citation reads a root document and holds the stale line already there", () => {
  const spec = { detector: "broken_citation" };
  const held = { "README.md": "See `src/gone.ts`.\n", "src/a.ts": "export const a = 1;\n" };
  assert.equal(pair(spec, held, held).present, false);
  const moved = { "README.md": "See `src/gone.ts` and `src/a.ts`.\n", "src/b.ts": "export const a = 1;\n" };
  assert.deepEqual(pair(spec, held, moved).sites, [{ document: "README.md", cites: "src/a.ts" }]);
});

test("broken_citation reads no document below the tree root", () => {
  const spec = { detector: "broken_citation" };
  const held = { "README.md": "ok\n" };
  assert.equal(pair(spec, held, { ...held, "docs/x.md": "See `src/gone.ts`.\n" }).present, false);
});

test("new_dead_symbol reports a definition nothing references any more", () => {
  const spec = { detector: "new_dead_symbol" };
  const held = {
    "src/a.rs": "fn helper() {}\nfn checksum() {}\npub fn run() { helper(); }\n",
  };
  assert.equal(pair(spec, held, held).present, false);
  const orphaned = { "src/a.rs": "fn helper() {}\nfn checksum() {}\npub fn run() {}\n" };
  assert.deepEqual(pair(spec, held, orphaned).sites, [{ symbol: "helper" }]);
});

test("unreached_member reports a family file nothing references", () => {
  const spec = { detector: "unreached_member", directory: "src/commands", suffix: "_command.rs" };
  const held = {
    "src/commands/add_command.rs": "pub fn run_add() {}\n",
    "src/registry.rs": "use crate::commands::add_command::run_add;\nfn d() { run_add(); }\n",
  };
  assert.equal(pair(spec, held, held).present, false);
  const orphaned = {
    "src/commands/add_command.rs": "pub fn run_add() {}\n",
    "src/commands/set_command.rs": "pub fn run_set() {}\n",
    "src/registry.rs": "use crate::commands::set_command::run_set;\nfn d() { run_set(); }\n",
  };
  assert.deepEqual(pair(spec, held, orphaned).sites, [{ file: "src/commands/add_command.rs" }]);
});

test("contract_break reports an item gone, a changed head and a widened shape", () => {
  const spec = { detector: "contract_break", entry: "src/index.ts" };
  const held = {
    "src/index.ts":
      "export interface Point {\n  lat: number;\n}\nexport function distance(a: Point): number {\n  return 1;\n}\n",
  };
  assert.equal(pair(spec, held, held).present, false);
  const renamed = {
    "src/index.ts":
      "export interface Point {\n  lat: number;\n}\nexport function far(a: Point): number {\n  return 1;\n}\n",
  };
  assert.equal(pair(spec, held, renamed).sites[0].item, "distance");
  const widened = {
    "src/index.ts":
      "export interface Point {\n  lat: number;\n  height: number;\n}\nexport function distance(a: Point): number {\n  return 1;\n}\n",
  };
  assert.equal(pair(spec, held, widened).sites[0].item, "Point");
  const added = {
    "src/index.ts":
      "export interface Point {\n  lat: number;\n}\nexport function distance(a: Point): number {\n  return 1;\n}\nexport function near(a: Point): number {\n  return 2;\n}\n",
  };
  assert.equal(pair(spec, held, added).present, false);
});

test("a declaration reflowed over several lines is the same contract", () => {
  const spec = { detector: "contract_break", entry: "src/index.ts" };
  const held = {
    "src/index.ts":
      "export interface Point {\n  lat: number;\n}\nexport function distance(a: Point, b: Point): number {\n  return 1;\n}\n",
  };
  const reflowed = {
    "src/index.ts":
      "export interface Point {\n  lat: number;\n}\nexport function distance(\n  a: Point,\n  b: Point,\n): number {\n  return 1;\n}\n",
  };
  assert.equal(pair(spec, held, reflowed).present, false, "reformatting is not a broken contract");
});

test("two files declaring one private name are two declarations", () => {
  const spec = { detector: "new_dead_symbol" };
  const held = {
    "src/a.rs": "fn helper() {}\npub fn one() { helper(); }\n",
    "src/b.rs": "fn helper() {}\npub fn two() { helper(); }\n",
  };
  assert.equal(pair(spec, held, held).present, false);
});

test("new_dead_symbol reports a TypeScript declaration left unexported and unreferenced", () => {
  const spec = { detector: "new_dead_symbol" };
  const held = {
    "src/a.ts":
      "function legacy() {}\nfunction helper() {}\nexport function run() {\n  helper();\n}\n",
  };
  assert.equal(pair(spec, held, held).present, false, "debt the base already held is not new");
  const orphaned = {
    "src/a.ts":
      "function legacy() {}\nfunction helper() {}\nconst LIMIT = 3;\ninterface Shape {}\nexport function run() {}\n",
  };
  assert.deepEqual(pair(spec, held, orphaned).sites, [
    { symbol: "LIMIT" },
    { symbol: "Shape" },
    { symbol: "helper" },
  ]);
});

test("new_dead_symbol ignores an exported, a nested or a TypeScript name another file declares", () => {
  const spec = { detector: "new_dead_symbol" };
  const held = { "src/a.ts": "export function run() {}\n" };
  const after = {
    "src/a.ts": "export function run() {\n  function inner() {}\n}\nexport class Kept {}\ntype Twice = number;\n",
    "src/b.ts": "type Twice = string;\n",
  };
  assert.equal(pair(spec, held, after).present, false);
});

test("unreached_member reports a TypeScript family file no other file imports", () => {
  const spec = { detector: "unreached_member", directory: "src/commands", suffix: ".command.ts" };
  const held = {
    "src/commands/add.command.ts": "export function runAdd() {}\n",
    "src/registry.ts": 'import { runAdd } from "./commands/add.command.js";\nrunAdd();\n',
  };
  assert.equal(pair(spec, held, held).present, false);
  const orphaned = {
    "src/commands/add.command.ts": "export function runAdd() {}\n",
    "src/commands/set.command.ts": "export function runSet() {}\n",
    "src/registry.ts": 'import { runAdd } from "./commands/add.command";\n// runSet is next\nrunAdd();\n',
  };
  assert.deepEqual(pair(spec, held, orphaned).sites, [{ file: "src/commands/set.command.ts" }]);
});

test("a commented-out or quoted import does not reach a TypeScript family file", () => {
  const spec = { detector: "unreached_member", directory: "src/commands", suffix: ".command.ts" };
  const held = {
    "src/commands/old.command.ts": "export function runOld() {}\n",
    "src/registry.ts": 'import { runOld } from "./commands/old.command.js";\nrunOld();\n',
  };
  const hidden = {
    "src/commands/old.command.ts": "export function runOld() {}\n",
    "src/registry.ts":
      '// import { runOld } from "./commands/old.command.js";\n/* import "./commands/old.command"; */\nconst note = \'import "./commands/old.command.js"\';\nconst later = `require("./commands/old.command")`;\n',
  };
  assert.deepEqual(pair(spec, held, hidden).sites, [{ file: "src/commands/old.command.ts" }]);
  const dynamic = { ...hidden, "src/lazy.ts": 'export const load = () => import("./commands/old.command.js");\n' };
  assert.equal(pair(spec, held, dynamic).present, false, "a dynamic import reaches it");
});

test("a detector the catalogue does not have is refused", () => {
  assert.throws(() => pair({ detector: "made_up" }, {}, {}), /no shortcut detector named made_up/);
});
