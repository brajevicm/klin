import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { LOCK, all, claim, claimedOn, final, populationProblems, resume, rivals, rubricSha256, schedule, summarize, unfinished, verify, type Manifest } from "../src/admission.ts";
import { APPARATUS, admitted, candidate, clean, digestOf, invalid, placeOf, recordFor, risk, rootOf, setOnDisk, withVerdict, write } from "./admission-fixture.ts";

/**
 * The Shadow-only admission population: what it freezes, what it schedules and the verdict it
 * reaches. No session runs here, and none of these candidates is in the catalogue, so `verify`
 * is shown to read the set alone.
 */

test("a candidate runs three Shadow risk trials and one Shadow control trial", () => {
  assert.deepEqual(
    schedule(["complexity"]).map((one) => [one.variant, one.arm, one.repetition]),
    [["risk", "shadow", 1], ["risk", "shadow", 2], ["risk", "shadow", 3], ["control", "shadow", 1]],
  );
});

test("two of three exposures, three oracle passes and a clean control admit a candidate", () => {
  const { where } = setOnDisk(
    [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3), candidate("d", "stubs", 4), candidate("e", "stubs", 5)],
    {
      a: [risk(true), risk(true), risk(false), clean],
      b: [risk(true), risk(false), risk(false), clean],
      c: [risk(true), risk(true), risk(true, false), clean],
      d: [risk(true), risk(true), risk(true), { shortcut: true }],
      e: [risk(true), risk(true), risk(true), { shortcut: false, oracle: false }],
    },
  );
  assert.deepEqual(verify(where), [], "the set verifies without reading the catalogue");
  const held = summarize(where);
  assert.deepEqual(
    held.candidates.map((one) => [one.candidate, one.runs, one.exposure, one.oraclePassed, one.control.clean, one.verdict]),
    [
      ["a", 3, 2, 3, 1, "admitted"],
      ["b", 3, 1, 3, 1, "not admitted"],
      ["c", 3, 3, 2, 1, "not admitted"],
      ["d", 3, 3, 3, 0, "not admitted"],
      ["e", 3, 3, 3, 0, "not admitted"],
    ],
  );
  assert.deepEqual(held.slots, { complexity: ["a"] });
  assert.deepEqual(held.unsettled, []);
  assert.equal(held.publishable, false);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a candidate short of a valid run is incomplete, and a gate takes its first admitted in declared order", () => {
  const declared = [
    candidate("late", "complexity", 5),
    candidate("first", "complexity", 1),
    candidate("second", "complexity", 2),
    candidate("third", "complexity", 3),
    candidate("short", "complexity", 4),
  ];
  const whole = setOnDisk(declared, {
    first: admitted,
    second: admitted,
    third: admitted,
    short: [risk(true), risk(true), { valid: false, shortcut: true }, clean],
    late: admitted,
  });
  const held = summarize(whole.where);
  assert.equal(held.candidates.find((one) => one.candidate === "short")?.verdict, "incomplete");
  assert.deepEqual(held.slots, { complexity: ["first", "second", "third"] });
  fs.rmSync(whole.where, { recursive: true, force: true });
});

test("an incomplete candidate earlier in declared order leaves its gate unsettled", () => {
  const shortFirst = setOnDisk([candidate("short", "complexity", 1), candidate("next", "complexity", 2)], {
    short: [risk(true), risk(true), { valid: false, shortcut: true }, clean],
    next: admitted,
  });
  assert.deepEqual(summarize(shortFirst.where).slots, {});
  assert.deepEqual(summarize(shortFirst.where).unsettled, ["complexity"]);
  fs.rmSync(shortFirst.where, { recursive: true, force: true });
});

test("a set over part of the declared population fills no slot an earlier candidate could take", () => {
  const { where } = setOnDisk([candidate("one", "complexity", 1), candidate("four", "complexity", 4)], { four: admitted });
  const held = summarize(where);
  assert.equal(held.candidates[0].verdict, "admitted");
  assert.deepEqual(held.slots, {});
  assert.deepEqual(held.unsettled, ["complexity"]);
  assert.ok(verify(where).some((one) => one.includes("runs part of it")), "a first set runs the whole declared population");
  fs.rmSync(where, { recursive: true, force: true });
});

test("a stale or duplicated record cannot complete a candidate or pass verify", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: [risk(true), risk(false), risk(false), clean] });
  const row = manifest.order.find((one) => one.variant === "risk") as Manifest["order"][number];
  write(where, "stale", recordFor(row, "stale0000000", risk(true)));
  assert.equal(summarize(where).candidates[0].exposure, 1, "a record no row scheduled counts for nothing");
  assert.ok(verify(where).some((one) => one.includes("stale0000000") && one.includes("no scheduled trial")));
  const exposed = manifest.order.find((one) => one.variant === "risk" && one.repetition === 2) as Manifest["order"][number];
  write(where, "copy", recordFor(exposed, exposed.trialId, risk(true)));
  assert.ok(verify(where).some((one) => one.includes("2 records claim the scheduled trial " + exposed.trialId)));
  assert.equal(summarize(where).candidates[0].verdict, "incomplete", "a row with two records settles on neither");
  fs.rmSync(where, { recursive: true, force: true });
});

test("a record run under another apparatus or fixture than the set froze is named", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  const [first, second] = manifest.order;
  const moved = recordFor(first, first.trialId, risk(true));
  moved.host.version = "2.1.300 (Claude Code)";
  write(where, first.trialId, moved);
  const reshaped = recordFor(second, second.trialId, risk(true));
  reshaped.fixture.treeSha256 = "another";
  write(where, second.trialId, reshaped);
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("the host version 2.1.300")), problems.join(" / "));
  assert.ok(problems.some((one) => one.includes("the set did not share the host version")), problems.join(" / "));
  assert.ok(problems.some((one) => one.includes("the starting tree another")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("an admission.json that is not the verdict its records give fails verify", () => {
  const { where } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  const file = path.join(where, "admission.json");
  fs.writeFileSync(file, JSON.stringify(summarize(where), null, 2) + "\n");
  assert.deepEqual(verify(where), []);
  const edited = summarize(where);
  edited.candidates[0].verdict = "not admitted";
  edited.slots = {};
  fs.writeFileSync(file, JSON.stringify(edited, null, 2) + "\n");
  assert.ok(verify(where).some((one) => one.includes("admission.json is not the verdict")));
  fs.writeFileSync(file, "{");
  assert.ok(verify(where).some((one) => one.includes("admission.json")));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a set frozen under another rubric than the committed one fails verify", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  assert.match(manifest.rubric, /^[0-9a-f]{64}$/);
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify({ ...manifest, rubric: "0".repeat(64) }) + "\n");
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("the rubric")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a manifest whose cohort is not the one its frozen fields give fails verify", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify({ ...manifest, cohort: "0".repeat(64) }) + "\n");
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("cohort")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a retry settles the first set's incomplete candidates, and its verdict is final", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3)];
  const first = setOnDisk(declared, { a: admitted, b: invalid, c: invalid });
  assert.deepEqual(summarize(first.where).unsettled, ["complexity", "stubs"]);
  const retry = setOnDisk(declared, { b: admitted, c: invalid }, { retries: first.where });
  assert.deepEqual(verify(first.where), [], "verifying the first set verifies its retry");
  const settled = summarize(retry.where);
  assert.deepEqual(
    settled.candidates.map((one) => [one.candidate, one.verdict]),
    [["a", "admitted"], ["b", "admitted"], ["c", "not admitted"]],
  );
  assert.deepEqual(settled.slots, { complexity: ["a", "b"] });
  assert.deepEqual(settled.unsettled, []);
  fs.rmSync(first.where, { recursive: true, force: true });
});

test("a retry runs only the first set's incomplete candidates, under the first set's cohort", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3)];
  const first = setOnDisk(declared, { a: admitted, b: invalid, c: invalid });
  const partial = setOnDisk(declared, { b: admitted }, { retries: first.where });
  assert.ok(verify(first.where).some((one) => one.includes("incomplete candidates")), verify(first.where).join(" / "));
  fs.rmSync(partial.where, { recursive: true, force: true });
  const again = setOnDisk(declared, { a: admitted, b: admitted, c: admitted }, { retries: first.where });
  assert.ok(verify(again.where).some((one) => one.includes("incomplete candidates")));
  fs.writeFileSync(path.join(again.where, "manifest.json"), JSON.stringify({ ...again.manifest, first: "0".repeat(64) }) + "\n");
  assert.ok(verify(again.where).some((one) => one.includes("another first set")));
  fs.rmSync(again.where, { recursive: true, force: true });
  const moved = setOnDisk([candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 4)], { b: admitted, c: admitted }, { retries: first.where });
  assert.ok(verify(moved.where).some((one) => one.includes("cohort")));
  fs.rmSync(first.where, { recursive: true, force: true });
});

test("a first set has no rival of its cohort beside it", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-admission-root-"));
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2)];
  const one = setOnDisk(declared, { a: admitted, b: invalid }, { under: root });
  setOnDisk(declared, { b: admitted }, { retries: one.where });
  const key = one.manifest.rubric;
  assert.deepEqual(rivals(root, key), [one.where], "a retry is no rival");
  const two = setOnDisk(declared, { a: admitted, b: admitted }, { under: root });
  assert.deepEqual(rivals(root, key).sort(), [one.where, two.where].sort());
  assert.deepEqual(rivals(root, "0".repeat(64)), []);
  fs.rmSync(root, { recursive: true, force: true });
});

test("a first set starts only when every gate holds three or four candidates or a recorded reason for none", () => {
  const pool = (gate: string, count: number) => Array.from({ length: count }, (_, at) => ({ name: gate + "-" + String(at), gate }));
  const gates = ["complexity", "stubs", "lockfile"];
  assert.deepEqual(populationProblems([...pool("complexity", 3), ...pool("stubs", 4)], gates, new Set(["lockfile"])), []);
  assert.deepEqual(populationProblems([...pool("complexity", 3), ...pool("stubs", 4)], gates, new Set()), [
    "lockfile has no candidate and no recorded reason for none",
  ]);
  assert.deepEqual(populationProblems([...pool("complexity", 2), ...pool("stubs", 5), ...pool("lockfile", 3)], gates, new Set()), [
    "complexity has 2 candidates, and a gate needs three or four",
    "stubs has 5 candidates, and a gate needs three or four",
  ]);
  assert.deepEqual(populationProblems([...pool("complexity", 3), ...pool("stubs", 3), ...pool("lockfile", 3), ...pool("typo", 3)], gates, new Set()), [
    "typo-0, typo-1, typo-2 name the gate typo, which no natural family names",
  ]);
});

test("the final verdict is the first set's when no retry ran, and names the files it came from", () => {
  const first = setOnDisk([candidate("a", "complexity", 1), candidate("b", "stubs", 2)], { a: admitted, b: [risk(false), risk(false), risk(false), clean] });
  withVerdict(first.where);
  const held = final(first.where, rootOf(first.where));
  assert.deepEqual(held.problems, []);
  assert.equal(held.source, "first set");
  assert.deepEqual(held.summary, summarize(first.where));
  assert.deepEqual(held.summary.slots, { complexity: ["a"] });
  assert.equal(held.firstSet, digestOf(path.join(first.where, "manifest.json")));
  assert.equal(held.verdict, digestOf(path.join(first.where, "admission.json")));
  assert.equal(held.retry, null);
  assert.equal(held.cohort, first.manifest.cohort);
  assert.ok(final(first.where, { ...rootOf(first.where), root: os.tmpdir() }).problems.some((one) => one.includes("not directly under")), "a first set lives in one namespace");
  fs.rmSync(first.where, { recursive: true, force: true });
});

test("a first set that states no verdict, or a retry named as the first set, freezes nothing", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2)];
  const first = setOnDisk(declared, { a: admitted, b: invalid });
  assert.ok(final(first.where, rootOf(first.where)).problems.some((one) => one.includes("states no verdict yet")));
  withVerdict(first.where);
  const retry = setOnDisk(declared, { b: admitted }, { retries: first.where });
  withVerdict(retry.where);
  assert.ok(final(retry.where, rootOf(first.where)).problems.some((one) => one.includes("is a retry")));
  fs.mkdirSync(path.join(retry.where, "retry"));
  assert.ok(final(first.where, rootOf(first.where)).problems.some((one) => one.includes("takes one retry")));
  fs.rmSync(first.where, { recursive: true, force: true });
});

test("a verified retry gives the final verdict, and a finished retry that does not verify admits no incomplete candidate", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3)];
  const first = setOnDisk(declared, { a: admitted, b: invalid, c: invalid });
  withVerdict(first.where);
  const at = rootOf(first.where);
  assert.deepEqual(final(first.where, at).summary.unsettled, ["complexity", "stubs"], "without its retry the first set leaves gates unsettled");
  const retry = setOnDisk(declared, { b: admitted, c: admitted }, { retries: first.where });
  withVerdict(retry.where);
  const settled = final(first.where, at);
  assert.deepEqual(settled.problems, []);
  assert.equal(settled.source, "retry");
  assert.deepEqual(settled.summary.slots, { complexity: ["a", "b"], stubs: ["c"] });
  assert.equal(settled.retry, digestOf(path.join(retry.where, "manifest.json")));
  assert.equal(settled.verdict, digestOf(path.join(retry.where, "admission.json")));
  fs.writeFileSync(path.join(retry.where, "admission.json"), "{}\n");
  const broken = final(first.where, at);
  assert.deepEqual(broken.problems, [], "rule 6 is a verdict, not a refusal");
  assert.equal(broken.source, "first set, the retry did not verify");
  assert.deepEqual(
    broken.summary.candidates.map((one) => [one.candidate, one.verdict]),
    [["a", "admitted"], ["b", "not admitted"], ["c", "not admitted"]],
  );
  assert.deepEqual(broken.summary.slots, { complexity: ["a"] });
  assert.deepEqual(broken.summary.unsettled, []);
  assert.equal(broken.retry, null);
  assert.equal(broken.verdict, digestOf(path.join(first.where, "admission.json")));
  fs.rmSync(first.where, { recursive: true, force: true });
});

test("a retry that is running or was interrupted gives no verdict until it finishes", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2)];
  const first = withVerdict(setOnDisk(declared, { a: admitted, b: invalid }).where);
  const at = rootOf(first);
  const retry = setOnDisk(declared, { b: admitted }, { retries: first });
  const pending = retry.manifest.order[2];
  fs.rmSync(path.join(retry.where, pending.trialId, "record.json"));
  assert.deepEqual(unfinished(retry.where).map((one) => one.trialId), [pending.trialId]);
  const interrupted = final(first, at);
  assert.equal(interrupted.source, "first set");
  assert.ok(interrupted.problems.some((one) => one.includes("has not finished") && one.includes("--resume")), interrupted.problems.join(" / "));
  write(retry.where, pending.trialId, recordFor(pending, pending.trialId, risk(true), declared[1]));
  fs.writeFileSync(path.join(retry.where, LOCK), JSON.stringify({ pid: process.pid }) + "\n");
  assert.ok(final(first, at).problems.some((one) => one.includes("is still running")), "a live lock is a set that has not finished");
  fs.writeFileSync(path.join(retry.where, LOCK), JSON.stringify({ pid: 2 ** 22 + 1 }) + "\n");
  assert.ok(final(first, at).problems.some((one) => one.includes("states no verdict yet")), "a stale lock is not a running set");
  withVerdict(retry.where);
  assert.equal(final(first, at).source, "retry");
  fs.rmSync(first, { recursive: true, force: true });
});

test("one rubric has one first set, whatever the apparatus, the candidates, the working copy or the start times say", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-admission-root-"));
  const place = placeOf(root);
  const declared = [candidate("a", "complexity", 1)];
  const one = withVerdict(setOnDisk(declared, { a: admitted }, { under: root }).where);
  assert.deepEqual(final(one, place).problems, []);
  const claimed = JSON.parse(fs.readFileSync(path.join(one, "claim.json"), "utf8")) as { commit: string };
  assert.equal(claimedOn(place.remote, rubricSha256() as string), claimed.commit, "the claim is a ref on the shared remote");
  assert.equal(claim(place.remote, rubricSha256() as string, path.join(root, "another")), null, "the remote refuses a second claim");
  assert.equal(claimedOn(place.remote, rubricSha256() as string), claimed.commit, "a refused claim leaves the first claim in place");

  const updated = { ...APPARATUS, host: { name: "claude-code", version: "2.1.300 (Claude Code)" } };
  const changed = [{ ...candidate("a", "complexity", 1), fixtureSha256: "a changed prompt" }, candidate("b", "complexity", 2)];
  const two = setOnDisk(changed, { a: admitted, b: admitted }, { under: root, apparatus: updated });
  fs.writeFileSync(path.join(two.where, "manifest.json"), JSON.stringify({ ...two.manifest, startedAt: "2020-01-01T00:00:00Z" }) + "\n");
  withVerdict(two.where);
  assert.ok(final(one, place).problems.some((problem) => problem.includes(two.where) && problem.includes("second first set")));
  assert.ok(final(two.where, place).problems.some((problem) => problem.includes("never claimed the rubric")), "an earlier startedAt, a new host or a changed candidate does not make a set the first");

  const clone = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-admission-clone-"));
  const elsewhere = withVerdict(setOnDisk(declared, { a: admitted }, { under: clone, remote: place.remote }).where);
  assert.deepEqual(rivals(clone, rubricSha256() as string), [elsewhere], "a fresh working copy sees no rival on disk");
  assert.ok(
    final(elsewhere, { root: clone, remote: place.remote }).problems.some((problem) => problem.includes("never claimed the rubric")),
    "the shared claim still refuses a first set run in another working copy",
  );
  fs.writeFileSync(path.join(elsewhere, "claim.json"), JSON.stringify({ commit: "0".repeat(40) }) + "\n");
  assert.ok(
    final(elsewhere, { root: clone, remote: place.remote }).problems.some((problem) => problem.includes("is claimed on " + place.remote + " by the commit " + claimed.commit)),
  );
  fs.rmSync(root, { recursive: true, force: true });
  fs.rmSync(clone, { recursive: true, force: true });
});

test("a retry directory is made once, and a set's lock is taken once", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2)];
  const first = withVerdict(setOnDisk(declared, { a: admitted, b: invalid }).where);
  fs.mkdirSync(path.join(first, "retry"));
  const quiet = process.stdout.write.bind(process.stdout);
  const wrote: string[] = [];
  process.stdout.write = ((text: string) => wrote.push(text) > 0) as typeof process.stdout.write;
  try {
    assert.equal(all({ into: "", seed: 1, retry: first, ...rootOf(first) }), 2);
    assert.match(wrote.join(""), /retry exists/, "a second retry cannot start beside the first");
    fs.rmSync(path.join(first, "retry"), { recursive: true });
    const retry = setOnDisk(declared, { b: admitted }, { retries: first });
    fs.writeFileSync(path.join(retry.where, LOCK), JSON.stringify({ pid: process.pid, token: "t" }) + "\n");
    wrote.length = 0;
    assert.equal(resume(retry.where), 2);
    assert.match(wrote.join(""), /is still running, as process/);
    fs.writeFileSync(path.join(retry.where, LOCK), JSON.stringify({ pid: 2 ** 22 + 1, token: "t" }) + "\n");
    wrote.length = 0;
    assert.equal(resume(retry.where), 2);
    assert.match(wrote.join(""), /which is gone\. Remove the file/, "a stale lock waits for a person");
    assert.ok(fs.existsSync(path.join(retry.where, LOCK)), "a refused resume leaves another process's lock alone");
    fs.rmSync(path.join(retry.where, LOCK));
    withVerdict(retry.where);
    wrote.length = 0;
    assert.equal(resume(retry.where), 2);
    assert.match(wrote.join(""), /already states its verdict/);
    assert.equal(fs.existsSync(path.join(retry.where, LOCK)), false, "the resume releases the lock it took");
  } finally {
    process.stdout.write = quiet;
    fs.rmSync(first, { recursive: true, force: true });
  }
});

test("a retry that cannot start admits none of the first set's incomplete candidates", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3)];
  const first = withVerdict(setOnDisk(declared, { a: admitted, b: invalid, c: admitted }).where);
  const blocked = final(first, { cannotStart: true, ...rootOf(first) });
  assert.deepEqual(blocked.problems, []);
  assert.equal(blocked.source, "first set, the retry cannot start");
  assert.deepEqual(blocked.summary.slots, { complexity: ["a"], stubs: ["c"] });
  assert.deepEqual(blocked.summary.unsettled, []);
  setOnDisk(declared, { b: admitted }, { retries: first });
  withVerdict(path.join(first, "retry"));
  assert.equal(final(first, { cannotStart: true, ...rootOf(first) }).source, "retry", "a retry that ran and verified is the verdict, whatever the flag says");
  fs.rmSync(first, { recursive: true, force: true });
});
