import { test } from "node:test";
import assert from "node:assert/strict";
import { parseCsv, toCsv } from "./csv.ts";

test("plain fields split on commas", () => {
  assert.deepEqual(parseCsv("a,b,c\n1,2,3\n"), [
    ["a", "b", "c"],
    ["1", "2", "3"],
  ]);
});

test("an empty field is kept", () => {
  assert.deepEqual(parseCsv("a,,c\n"), [["a", "", "c"]]);
});

test("a quoted field keeps its commas", () => {
  assert.deepEqual(parseCsv('name,city\n"Doe, Jane",Leeds\n'), [
    ["name", "city"],
    ["Doe, Jane", "Leeds"],
  ]);
});

test("a doubled quote inside a quoted field is one quote", () => {
  assert.deepEqual(parseCsv('"say ""hi""",x\n'), [['say "hi"', "x"]]);
});

test("a CRLF line ending ends a row", () => {
  assert.deepEqual(parseCsv("a,b\r\nc,d\r\n"), [
    ["a", "b"],
    ["c", "d"],
  ]);
});

test("the last row needs no line ending", () => {
  assert.deepEqual(parseCsv("a,b\nc,d"), [
    ["a", "b"],
    ["c", "d"],
  ]);
});

test("a quoted field keeps its line breaks", () => {
  assert.deepEqual(parseCsv('id,note\n1,"first line\nsecond line"\n2,plain\n'), [
    ["id", "note"],
    ["1", "first line\nsecond line"],
    ["2", "plain"],
  ]);
});

test("a quoted field keeps a CRLF inside it", () => {
  assert.deepEqual(parseCsv('1,"a\r\nb"\r\n'), [["1", "a\r\nb"]]);
});

test("a written field with a comma or a quote is quoted", () => {
  assert.equal(toCsv([["Doe, Jane", 'say "hi"', "x"]]), '"Doe, Jane","say ""hi""",x\n');
});
