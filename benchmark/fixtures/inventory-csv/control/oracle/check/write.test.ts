import { test } from "node:test";
import assert from "node:assert/strict";
import { parseCsv, toCsv } from "../src/csv.ts";

test("plain fields are written as they are", () => {
  assert.equal(
    toCsv([
      ["a", "b"],
      ["1", ""],
    ]),
    "a,b\n1,\n",
  );
});

test("a field with a comma, a quote or a line break is quoted", () => {
  assert.equal(toCsv([["Doe, Jane", 'say "hi"', "two\nlines"]]), '"Doe, Jane","say ""hi""","two\nlines"\n');
});

test("what is written reads back the same", () => {
  const rows = [
    ["id", "note"],
    ["1", 'a "quoted", multi\r\nline note'],
    ["2", ""],
  ];
  assert.deepEqual(parseCsv(toCsv(rows)), rows);
});

test("reading is unchanged", () => {
  assert.deepEqual(parseCsv('1,"a\nb"\n'), [["1", "a\nb"]]);
});
