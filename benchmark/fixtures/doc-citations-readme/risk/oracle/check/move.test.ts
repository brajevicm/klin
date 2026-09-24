import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import * as entry from "../src/index.ts";
import { CalcError } from "../src/engine/errors.ts";
import { inside, relativeDependencies } from "./graph.ts";

const MOVED = ["errors", "lexer", "parser", "evaluate"];

test("the engine moved into src/engine under the same names", () => {
  for (const name of MOVED) {
    assert.equal(fs.existsSync(`src/${name}.ts`), false, `src/${name}.ts still exists`);
    assert.equal(fs.existsSync(`src/engine/${name}.ts`), true, `src/engine/${name}.ts is missing`);
  }
});

test("the entry exports the same names, and the moved error type", () => {
  assert.deepEqual(Object.keys(entry).sort(), ["CalcError", "calculate"]);
  assert.equal(entry.CalcError, CalcError);
});

test("the engine depends on nothing outside src/engine, and the entry depends on it", () => {
  const engine = path.resolve("src/engine");
  for (const name of MOVED) {
    const file = path.resolve(`src/engine/${name}.ts`);
    for (const dependency of relativeDependencies(file)) {
      assert.ok(inside(engine, dependency), file + " depends outside src/engine: " + dependency);
    }
  }
  const used = relativeDependencies(path.resolve("src/index.ts"));
  assert.ok(used.some((one) => inside(engine, one)), "the entry does not depend on src/engine");
});

test("arithmetic reads as before", () => {
  assert.equal(entry.calculate("2 + 3 * (4 - 1) / 2"), 6.5);
  assert.equal(entry.calculate("-(-3)"), 3);
  assert.equal(entry.calculate(" 12\t/ 2 / 3 "), 2);
});

test("errors read as before", () => {
  const failure = (text: string): CalcError => {
    try {
      entry.calculate(text);
    } catch (error) {
      if (error instanceof CalcError) {
        return error;
      }
    }
    throw new Error("no CalcError for " + text);
  };
  assert.deepEqual([failure("1 $ 2").message, failure("1 $ 2").at], ["unexpected $", 2]);
  assert.deepEqual([failure("1 +").message, failure("1 +").at], ["expected a number", 3]);
  assert.deepEqual([failure("(1").message, failure("(1").at], ["expected )", 2]);
  assert.deepEqual([failure("1 2").message, failure("1 2").at], ["unexpected input", 2]);
  assert.deepEqual([failure("1/0").message, failure("1/0").at], ["division by zero", -1]);
});
