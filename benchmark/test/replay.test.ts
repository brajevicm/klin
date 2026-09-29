import { test } from "node:test";
import assert from "node:assert/strict";
import { changesBefore, groupOf, journalRows } from "../src/replay.ts";

const derivedFloor = [
  { section: "complexity", key: "cc", value: 5, rule: "p95" },
  { section: "complexity", key: "lines", value: 25, rule: "p95" },
];

test("the ten changes start at the first commit before the cutoff and each takes the next as its base", () => {
  const walk = Array.from({ length: 13 }, (_, at) => ({ sha: `c${at}`, time: 100 - at, subject: `s${at}` }));
  const changes = changesBefore(walk, 99);
  assert.equal(changes?.length, 10);
  assert.deepEqual(changes?.[0], { head: "c2", base: "c3", subject: "s2" });
  assert.deepEqual(changes?.[9], { head: "c11", base: "c12", subject: "s11" });
  assert.equal(changesBefore(walk, 98), null);
});

test("a complexity finding is grouped by the ceiling it crossed", () => {
  const floor = { ceiling: "cc 5, lines 25", values: { cc: 2, lines: 31 }, matched: { file: "a", line: 1, text: "", values: { cc: 2, lines: 22 } } };
  assert.equal(groupOf("complexity", floor, derivedFloor), "lines at the floor");
  const percentile = { ceiling: "cc 7, lines 58", values: { cc: 9, lines: 10 }, matched: null };
  assert.equal(groupOf("complexity", percentile, derivedFloor), "cc at a derived percentile");
  const grew = { ceiling: "cc 5, lines 25", values: { cc: 2, lines: 266 }, matched: { file: "a", line: 1, text: "", values: { cc: 2, lines: 247 } } };
  assert.equal(groupOf("complexity", grew, derivedFloor), "a site the base held over the ceiling grew");
  assert.equal(groupOf("complexity", percentile, []), "pinned ceiling");
  assert.equal(groupOf("doc-size", { file: "CHANGELOG.md" }, []), "document CHANGELOG.md");
});

test("consecutive stops of one session with the same findings share a row", () => {
  const finding = { gate: "inventory", file: "tests/a.rs", line: 3, text: "fn a() {", outcome: "worsened", values: { missing: 1 } };
  const stop = (session: string, findings: object[], status = "FAIL") =>
    JSON.stringify({
      kind: "stop",
      session,
      time: 0,
      version: "0.3.0",
      status,
      window: { kind: "turn", before: "abc", how: "the turn stamp" },
      gates: [{ name: "inventory", status: findings.length > 0 ? "FAIL" : "ok" }],
      findings,
    });
  const lines = [
    JSON.stringify({ kind: "prompt", session: "s1", text: "delete the old test", time: 0 }),
    stop("s1", [finding]),
    stop("s2", [finding]),
    stop("s1", [finding]),
    stop("s1", [], "PASS"),
    stop("s1", [finding]),
  ];
  const rows = journalRows(lines);
  assert.deepEqual(
    rows.map((row) => [row.id, row.context.session, row.context.stops]),
    [
      ["J001", "s1", 2],
      ["J002", "s2", 1],
      ["J003", "s1", 1],
    ],
  );
  assert.equal(rows[0].context.prompt, "delete the old test");
});
