import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { bearing, distance, distanceInSpace } from "../src/index.ts";

// ponytail: this dependency-free parser covers the frozen interface/re-export shape; use a TS parser if that contract grows.
type Token = { kind: "word" | "string" | "punct"; line: number; value: string };

function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let line = 1;

  for (let at = 0; at < source.length; ) {
    const current = source[at];
    const next = source[at + 1];

    if (/\s/.test(current)) {
      if (current === "\n") line += 1;
      at += 1;
      continue;
    }

    if (current === "/" && next === "/") {
      at += 2;
      while (at < source.length && source[at] !== "\n") at += 1;
      continue;
    }

    if (current === "/" && next === "*") {
      at += 2;
      while (at < source.length && !(source[at] === "*" && source[at + 1] === "/")) {
        if (source[at] === "\n") line += 1;
        at += 1;
      }
      at += 2;
      continue;
    }

    if (current === "'" || current === '"' || current === "`") {
      const quote = current;
      const startLine = line;
      let value = "";
      at += 1;
      while (at < source.length && source[at] !== quote) {
        if (source[at] === "\\" && at + 1 < source.length) {
          value += source[at + 1];
          if (source[at + 1] === "\n") line += 1;
          at += 2;
        } else {
          if (source[at] === "\n") line += 1;
          value += source[at];
          at += 1;
        }
      }
      at += 1;
      tokens.push({ kind: "string", line: startLine, value });
      continue;
    }

    if (/[A-Za-z_$]/.test(current)) {
      const start = at;
      at += 1;
      while (at < source.length && /[A-Za-z0-9_$]/.test(source[at])) at += 1;
      tokens.push({ kind: "word", line, value: source.slice(start, at) });
      continue;
    }

    tokens.push({ kind: "punct", line, value: current });
    at += 1;
  }

  return tokens;
}

function hasReadingInterface(tokens: Token[]): boolean {
  for (let at = 0; at + 4 < tokens.length; at += 1) {
    if (
      tokens[at].value !== "export" ||
      tokens[at + 1].value !== "interface" ||
      tokens[at + 2].value !== "Reading" ||
      tokens[at + 3].value !== "{"
    ) {
      continue;
    }

    const fields = new Set<string>();
    let fieldCount = 0;
    for (let field = at + 4; field < tokens.length; field += 1) {
      if (tokens[field].value === "}") {
        return (
          fieldCount === 3 &&
          fields.size === 3 &&
          ["lat", "lon", "height"].every((name) => fields.has(name))
        );
      }
      if (tokens[field].value === ";" || tokens[field].value === ",") continue;
      if (
        tokens[field].kind !== "word" ||
        tokens[field + 1]?.value !== ":" ||
        tokens[field + 2]?.kind !== "word" ||
        tokens[field + 2]?.value !== "number"
      ) {
        return false;
      }

      fieldCount += 1;
      fields.add(tokens[field].value);
      const type = tokens[field + 2];
      const separator = tokens[field + 3];
      if (
        !separator ||
        (separator.value !== "}" &&
          separator.value !== ";" &&
          separator.value !== "," &&
          separator.line === type.line)
      ) {
        return false;
      }
      field += 2;
    }
  }

  return false;
}

function readingReexports(tokens: Token[]): string[] {
  const modules: string[] = [];
  for (let at = 0; at < tokens.length; at += 1) {
    if (tokens[at].value !== "export") continue;
    let cursor = at + 1;
    if (tokens[cursor]?.value === "type") cursor += 1;
    if (tokens[cursor]?.value !== "{") continue;
    cursor += 1;

    let exportsReading = false;
    while (cursor < tokens.length && tokens[cursor].value !== "}") {
      if (tokens[cursor].value === ",") {
        cursor += 1;
        continue;
      }
      if (tokens[cursor].value === "type") cursor += 1;
      const imported = tokens[cursor++];
      if (!imported || imported.kind !== "word") break;
      let exported = imported.value;
      if (tokens[cursor]?.value === "as") {
        exported = tokens[cursor + 1]?.value ?? "";
        cursor += 2;
      }
      if (exported === "Reading") exportsReading = true;
    }

    if (!exportsReading || tokens[cursor]?.value !== "}") continue;
    if (tokens[cursor + 1]?.value !== "from" || tokens[cursor + 2]?.kind !== "string") continue;
    modules.push(tokens[cursor + 2].value);
  }
  return modules;
}

function resolveModule(from: URL, specifier: string): URL | undefined {
  if (!specifier.startsWith(".")) return undefined;
  const candidates = [new URL(specifier, from)];
  if (!/\.[^/]+$/.test(specifier)) {
    candidates.push(new URL(`${specifier}.ts`, from));
    candidates.push(new URL(`${specifier}/index.ts`, from));
  }
  if (specifier.endsWith(".js")) candidates.push(new URL(`${specifier.slice(0, -3)}.ts`, from));
  return candidates.find((candidate) => fs.existsSync(fileURLToPath(candidate)));
}

function hasReadingContract(file: URL, seen = new Set<string>()): boolean {
  if (seen.has(file.href)) return false;
  seen.add(file.href);

  const tokens = tokenize(fs.readFileSync(fileURLToPath(file), "utf8"));
  if (hasReadingInterface(tokens)) return true;
  return readingReexports(tokens).some((specifier) => {
    const target = resolveModule(file, specifier);
    return target !== undefined && hasReadingContract(target, seen);
  });
}

test("the published Reading shape is declared", () => {
  assert.equal(hasReadingContract(new URL("../src/index.ts", import.meta.url)), true);
});

test("the Reading oracle parses the exact shape", () => {
  const accepted = `export interface Reading {
    /* comments and reordered members are fine */
    height: number
    lat: number,
    lon: number;
  }`;
  const rejected = [
    "// export interface Reading { lat: number; lon: number; height: number; }",
    "/* export interface Reading { lat: number; lon: number; height: number; } */",
    'const fake = "export interface Reading { lat: number; lon: number; height: number; }";',
    "export interface Reading { lat: number | string; lon: number; height: number; }",
    "export interface Reading { lat: number; lon: number; height: number; label: string; }",
    "export interface Reading { lat: number; lon: number; }",
  ];

  assert.equal(hasReadingInterface(tokenize(accepted)), true);
  for (const source of rejected) assert.equal(hasReadingInterface(tokenize(source)), false);
});

test("the Reading oracle follows an entry-point re-export", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-reading-"));
  fs.writeFileSync(path.join(root, "index.ts"), 'export { Reading } from "./reading.ts";');
  fs.writeFileSync(
    path.join(root, "reading.ts"),
    "export interface Reading { lat: number; lon: number; height: number; }",
  );
  try {
    assert.equal(hasReadingContract(pathToFileURL(path.join(root, "index.ts"))), true);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("space distance combines the ground leg and height difference", () => {
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 100 }, { lat: 0, lon: 0, height: 103 }), 3);
  assert.equal(
    distanceInSpace({ lat: 51.5, lon: 0, height: 1000 }, { lat: 51.5, lon: 1, height: 1334 }),
    100154,
  );
  assert.equal(
    distanceInSpace({ lat: 0, lon: 0, height: 1000 }, { lat: 1, lon: 0, height: 1334 }),
    111196,
  );
});

test("the ground distance is what it was", () => {
  assert.equal(distance({ lat: 0, lon: 0 }, { lat: 1, lon: 0 }), 111195);
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 1 }), 100153);
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 0 }), 0);
});

test("the direction is what it was", () => {
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: 0, lon: 1 }), 90);
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: -1, lon: 0 }), 180);
});
