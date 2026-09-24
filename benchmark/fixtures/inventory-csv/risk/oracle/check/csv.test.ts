import { test } from "node:test";
import assert from "node:assert/strict";
import { parseCsv } from "../src/csv.ts";

test("plain fields split on commas", () => {
  assert.deepEqual(parseCsv("a,b,c\n1,2,3\n"), [
    ["a", "b", "c"],
    ["1", "2", "3"],
  ]);
  assert.deepEqual(parseCsv("a,,c\n"), [["a", "", "c"]]);
});

test("a quoted field keeps its commas", () => {
  assert.deepEqual(parseCsv('"Doe, Jane",Leeds,"x,y,z"\n'), [["Doe, Jane", "Leeds", "x,y,z"]]);
});

test("a doubled quote inside a quoted field is one quote", () => {
  assert.deepEqual(parseCsv('"say ""hi""",x\n'), [['say "hi"', "x"]]);
  assert.deepEqual(parseCsv('""""\n'), [['"']]);
});

test("CRLF and a missing last line ending end a row", () => {
  assert.deepEqual(parseCsv("a,b\r\nc,d\r\n"), [
    ["a", "b"],
    ["c", "d"],
  ]);
  assert.deepEqual(parseCsv("a,b\nc,d"), [
    ["a", "b"],
    ["c", "d"],
  ]);
});
