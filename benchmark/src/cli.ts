import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { ARMS, catalogue, cells, families, variantNames } from "./catalogue.ts";
import * as selftest from "./selftest.ts";
import * as calibrate from "./calibrate.ts";
import * as admission from "./admission.ts";
import * as probe from "./probe.ts";
import * as report from "./report.ts";
import * as evidence from "./evidence.ts";
import * as round from "./round.ts";
import * as seeded from "./seeded.ts";
import * as audit from "./audit.ts";
import * as toolchain from "./toolchain.ts";
import * as worksheet from "./worksheet.ts";
import * as synthesis from "./synthesis.ts";

const USAGE = `klin Shadow/Active benchmark

  node benchmark/src/cli.ts list
  node benchmark/src/cli.ts toolchain
  node benchmark/src/cli.ts selftest [family ...]
  node benchmark/src/cli.ts run <family> <variant> <active|shadow> [--into DIR]
  node benchmark/src/cli.ts probe [family]
  node benchmark/src/cli.ts calibrate [--into DIR] [--seed N] [--only family,...] [--population admission]
  node benchmark/src/cli.ts protocol [--seed N] [--write]
  node benchmark/src/cli.ts plan [--into DIR] [--seed N] [--population seeded [--families a,b] [--repetitions N]]
  node benchmark/src/cli.ts execute <round-dir> --manifest-sha256 HEX
  node benchmark/src/cli.ts seeded-plan [--into DIR] [--seed N] [--families a,b] [--repetitions N]
  node benchmark/src/cli.ts seeded-execute <round-dir> --manifest-sha256 HEX
  node benchmark/src/cli.ts verify <records-dir>
  node benchmark/src/cli.ts report <records-dir> [--out FILE]
  node benchmark/src/cli.ts scorecard <round-dir> [--out FILE]
  node benchmark/src/cli.ts audit <evidence-dir> [--archive FILE] [--out FILE]
  node benchmark/src/cli.ts label-prepare <v1-evidence> <v2-evidence> --archive-v1 FILE --archive-v2 FILE --into DIR
  node benchmark/src/cli.ts label-synthesize <labeling-dir> <v1-evidence> <v2-evidence>
  node benchmark/src/cli.ts evidence-prepare <runs-dir> --into DIR --archive FILE
  node benchmark/src/cli.ts evidence-verify <evidence-dir> [--archive FILE]

Environment:
  KLIN_BIN              the klin binary under test, default target/release/klin
  KLIN_BENCH_MODEL      the model the host runs, default sonnet
  KLIN_BENCH_BUDGET     dollars per trial, default 5
  KLIN_BENCH_TIMEOUT_MS wall clock per trial, default 1800000
  KLIN_BENCH_WORK       where subject workspaces are materialized
`;

function flag(args: string[], name: string, fallback: string): string {
  const at = args.indexOf(name);
  return at >= 0 && at + 1 < args.length ? args[at + 1] : fallback;
}

function seededDesign(args: string[]): seeded.Design {
  const named = flag(args, "--families", "");
  return {
    families: named === "" ? seeded.everyFamily().families : named.split(",").sort(),
    repetitions: Number(flag(args, "--repetitions", "1")),
  };
}

/**
 * The arguments that are not a flag or a flag's value.
 *
 * A flag's value does not start with a dash, so filtering on the dash alone reads `--into DIR` as
 * a family name, and `probe --into /tmp/x` looked for a family called `/tmp/x`.
 */
export function positionals(args: string[]): string[] {
  const kept: string[] = [];
  for (let at = 0; at < args.length; at += 1) {
    if (args[at].startsWith("-")) {
      at += 1;
      continue;
    }
    kept.push(args[at]);
  }
  return kept;
}

/**
 * Every way a `run` command line names something the catalogue does not have.
 *
 * Where a family was named, the variant is held to that family's own list rather than a global
 * one. A planted variant is shipped by the one family it was built for, so `run` is the only way
 * to address one and another family refuses it by name. Where no family was named, the variant is
 * held to every variant any family ships, so a command line with three wrong parts still names
 * all three at once.
 */
export function wrongArguments(family: string, variant: string, arm: string): string[] {
  const found = catalogue()[family];
  const named = found
    ? (variantNames(found) as string[])
    : [...new Set(Object.values(catalogue()).flatMap((one) => variantNames(one) as string[]))];
  return [
    found ? "" : "no family named " + String(family),
    named.includes(variant)
      ? ""
      : found
        ? family + " ships no variant named " + String(variant) + ", only " + named.join(", ")
        : "no variant named " + String(variant),
    (ARMS as readonly string[]).includes(arm) ? "" : "no arm named " + String(arm),
  ].filter((one) => one.length > 0);
}

function list(): number {
  for (const [name, family] of Object.entries(catalogue())) {
    const candidate = family.spec.candidate === undefined ? "" : "candidate " + String(family.spec.candidate) + ", ";
    process.stdout.write(
      name.padEnd(16) + family.spec.language.padEnd(12) + candidate + family.spec.summary + "\n",
    );
    for (const variant of Object.values(family.variants)) {
      process.stdout.write(
        "  " +
          variant.name.padEnd(10) +
          variant.taskId +
          "  " +
          variant.shortcut.detector +
          "\n",
      );
    }
  }
  process.stdout.write("\n" + String(cells().length) + " calibration cells\n");
  return 0;
}

function toolchainStatus(): number {
  const problem = toolchain.requirement();
  process.stdout.write((problem || toolchain.describe()) + "\n");
  return problem === "" ? 0 : 2;
}

function runSelftest(only: string[]): number {
  const cases = selftest.run(only);
  let failed = 0;
  for (const one of cases) {
    const mark = one.passed ? "ok  " : "FAIL";
    if (!one.passed) {
      failed += 1;
    }
    process.stdout.write(
      mark + " " + one.family.padEnd(14) + one.variant.padEnd(9) + one.name + "\n",
    );
    if (!one.passed) {
      process.stdout.write("     " + one.detail.replace(/\n/g, "\n     ") + "\n");
    }
  }
  process.stdout.write(
    "\n" + String(cases.length - failed) + " passed, " + String(failed) + " failed\n",
  );
  return failed === 0 ? 0 : 1;
}

function kindOf(directory: string): string {
  const file = path.join(directory, "manifest.json");
  if (!fs.existsSync(file)) {
    return "calibration";
  }
  return String((JSON.parse(fs.readFileSync(file, "utf8")) as { kind?: string }).kind ?? "calibration");
}

function populationOf(directory: string): string {
  const manifest = path.join(directory, "manifest.json");
  return fs.existsSync(manifest)
    ? String((JSON.parse(fs.readFileSync(manifest, "utf8")) as { population?: string }).population ?? "")
    : "";
}

function verify(directory: string): number {
  const population = populationOf(directory);
  const problems =
    kindOf(directory) === admission.POPULATION
      ? admission.verify(directory)
      : kindOf(directory) === "publishable"
      ? population === "seeded"
        ? seeded.verify(directory)
        : round.verify(directory)
      : calibrate.verify(directory);
  for (const line of problems) {
    process.stdout.write(line + "\n");
  }
  process.stdout.write(
    problems.length === 0
      ? "every record holds the contract\n"
      : String(problems.length) + (problems.length === 1 ? " problem\n" : " problems\n"),
  );
  return problems.length === 0 ? 0 : 1;
}

function evidencePrepare(source: string, args: string[]): number {
  const into = flag(args, "--into", "");
  const archive = flag(args, "--archive", "");
  if (source === "" || into === "" || archive === "") {
    process.stdout.write("evidence-prepare needs a runs directory, --into DIR and --archive FILE\n\n" + USAGE);
    return 2;
  }
  try {
    const read = evidence.prepare(source, into, archive);
    process.stdout.write(
      "prepared " + read.kind + " evidence with " + String(read.attempts) + " attempts\n",
    );
    return 0;
  } catch (why) {
    process.stdout.write(String(why) + "\n");
    return 2;
  }
}

function evidenceVerify(directory: string, args: string[]): number {
  if (directory === "") {
    process.stdout.write("evidence-verify needs an evidence directory\n\n" + USAGE);
    return 2;
  }
  const problems = evidence.verify(directory, flag(args, "--archive", ""));
  for (const problem of problems) {
    process.stdout.write(problem + "\n");
  }
  process.stdout.write(
    problems.length === 0
      ? "evidence is intact\n"
      : String(problems.length) + (problems.length === 1 ? " problem\n" : " problems\n"),
  );
  return problems.length === 0 ? 0 : 1;
}

function auditEvidence(directory: string, args: string[]): number {
  if (directory === "") {
    process.stdout.write("audit needs an evidence directory\n\n" + USAGE);
    return 2;
  }
  try {
    const text = audit.write(directory, flag(args, "--archive", ""));
    const out = flag(args, "--out", "");
    if (out) {
      fs.writeFileSync(out, text);
    } else {
      process.stdout.write(text);
    }
    return 0;
  } catch (why) {
    process.stdout.write(String(why) + "\n");
    return 2;
  }
}

function prepareWorksheet(args: string[]): number {
  const [v1, v2] = positionals(args);
  const into = flag(args, "--into", "");
  const archiveV1 = flag(args, "--archive-v1", "");
  const archiveV2 = flag(args, "--archive-v2", "");
  if (!v1 || !v2 || into === "" || archiveV1 === "" || archiveV2 === "") {
    process.stdout.write(
      "label-prepare needs v1 and v2 evidence directories, --archive-v1 FILE, --archive-v2 FILE and --into DIR\n\n" + USAGE,
    );
    return 2;
  }
  try {
    const read = worksheet.prepare([
      { name: "v1", directory: v1, archive: archiveV1 },
      { name: "v2", directory: v2, archive: archiveV2 },
    ], into);
    process.stdout.write(
      "prepared blinded worksheet with " +
        String(read.counts.includedRecords) +
        " records, " +
        String(read.counts.signalOccurrences) +
        " signal occurrences and " +
        String(read.counts.worksheetRows) +
        " worksheet rows; stop at the human-labeling gate\n",
    );
    return 0;
  } catch (why) {
    process.stdout.write(String(why) + "\n");
    return 2;
  }
}

function synthesizeLabels(args: string[]): number {
  const [labeling, v1, v2] = positionals(args);
  if (!labeling || !v1 || !v2) {
    process.stdout.write("label-synthesize needs a labeling directory and the v1 and v2 evidence directories\n\n" + USAGE);
    return 2;
  }
  try {
    const read = synthesis.synthesize(labeling, { v1, v2 });
    process.stdout.write(
      "verified locked labels " + read.labelsSha256 + " and wrote synthesis.json and synthesis.md: " +
        synthesis.LABELS.map((label) => String(read.labels[label]) + " " + label).join(", ") +
        " over " + String(read.sites.reduce((sum, row) => sum + row.sites, 0)) + " sites\n",
    );
    return 0;
  } catch (why) {
    process.stdout.write(String(why) + "\n");
    return 2;
  }
}

export function main(argv: string[]): number {
  const [command, ...args] = argv;
  if (!command || command === "--help" || command === "-h") {
    process.stdout.write(USAGE);
    return 0;
  }
  if (command === "list") {
    return list();
  }
  if (command === "toolchain") {
    return toolchainStatus();
  }
  if (command === "selftest") {
    return runSelftest(positionals(args));
  }
  if (command === "run") {
    const [family, variant, arm] = args;
    const wrong = wrongArguments(family, variant, arm);
    if (wrong.length > 0) {
      process.stdout.write(wrong.join("\n") + "\n\n" + USAGE);
      return 2;
    }
    const into = flag(args, "--into", path.join(paths.RUNS, "ad-hoc"));
    return calibrate.one(family, variant, arm, into);
  }
  if (command === "probe") {
    const [family] = positionals(args);
    const found = families();
    if (family !== undefined && !(family in found)) {
      process.stdout.write("no family named " + family + "\n\n" + USAGE);
      return 2;
    }
    const named = family !== undefined ? [family] : probe.perLanguage(found);
    // A probe writes where `plan` looks and nowhere else. The verification composes the plane's
    // own path from the trial id, so a probe written elsewhere could authorize nothing.
    return Math.max(...named.map((one) => probe.run(one)));
  }
  if (command === "calibrate") {
    const population = flag(args, "--population", "");
    if (population !== "" && population !== admission.POPULATION) {
      process.stdout.write("calibrate knows no population named " + population + ", only " + admission.POPULATION + "\n");
      return 2;
    }
    if (population === admission.POPULATION) {
      return admission.all({
        into: flag(args, "--into", admission.directory()),
        only: flag(args, "--only", "").split(",").filter((one) => one.length > 0),
        seed: Number(flag(args, "--seed", "1")),
      });
    }
    return calibrate.all({
      into: flag(args, "--into", path.join(paths.RUNS, calibrate.stamp())),
      only: flag(args, "--only", "")
        .split(",")
        .filter((one) => one.length > 0),
      seed: Number(flag(args, "--seed", "1")),
    });
  }
  if (command === "protocol") {
    return round.protocol(Number(flag(args, "--seed", "1")), args.includes("--write"));
  }
  if (command === "plan") {
    if (flag(args, "--population", "") === "seeded") {
      return seeded.plan(flag(args, "--into", seeded.roundDirectory()), Number(flag(args, "--seed", "1")), seededDesign(args));
    }
    return round.plan(flag(args, "--into", round.roundDirectory()), Number(flag(args, "--seed", "1")));
  }
  if (command === "execute") {
    const approved = flag(args, "--manifest-sha256", "");
    if (!args[0] || args[0].startsWith("-") || approved === "") {
      process.stdout.write(
        "execute needs a planned round directory and --manifest-sha256, the digest the owner approved\n\n" + USAGE,
      );
      return 2;
    }
    return populationOf(args[0]) === "seeded"
      ? seeded.execute(args[0], approved)
      : round.execute(args[0], approved);
  }
  if (command === "seeded-plan") {
    return seeded.plan(flag(args, "--into", seeded.roundDirectory()), Number(flag(args, "--seed", "1")), seededDesign(args));
  }
  if (command === "seeded-execute") {
    const approved = flag(args, "--manifest-sha256", "");
    if (!args[0] || args[0].startsWith("-") || approved === "") {
      process.stdout.write(
        "seeded-execute needs a planned round directory and --manifest-sha256, the digest the owner approved\n\n" + USAGE,
      );
      return 2;
    }
    return seeded.execute(args[0], approved);
  }
  if (command === "verify") {
    return verify(args[0] ?? "");
  }
  if (command === "scorecard") {
    if (!args[0] || kindOf(args[0]) !== "publishable") {
      process.stdout.write("scorecard needs a publishable round directory\n\n" + USAGE);
      return 2;
    }
    if (populationOf(args[0]) === "seeded") {
      process.stdout.write("seeded rounds use report, not scorecard\n");
      return 2;
    }
    let card: round.Scorecard;
    try {
      card = round.scorecard(args[0]);
    } catch (why) {
      process.stdout.write(String(why) + "\n");
      return 2;
    }
    fs.writeFileSync(path.join(args[0], "scorecard.json"), JSON.stringify(card, null, 2) + "\n");
    const text = round.markdown(card);
    const out = flag(args, "--out", "");
    if (out) {
      fs.writeFileSync(out, text);
    } else {
      process.stdout.write(text);
    }
    return card.verification.length === 0 ? 0 : 1;
  }
  if (command === "audit") {
    return auditEvidence(args[0] ?? "", args.slice(1));
  }
  if (command === "label-synthesize") {
    return synthesizeLabels(args);
  }
  if (command === "label-prepare") {
    return prepareWorksheet(args);
  }
  if (command === "report") {
    const directory = args[0] ?? "";
    if (kindOf(directory) === admission.POPULATION) {
      process.stdout.write("an admission set's signals stay sealed until the result document; its verdicts are in admission.json\n");
      return 2;
    }
    const { text, problems } =
      populationOf(directory) === "seeded" ? seeded.report(directory) : { text: report.write(directory), problems: [] };
    const out = flag(args, "--out", "");
    if (out) {
      fs.writeFileSync(out, text);
    } else {
      process.stdout.write(text);
    }
    if (problems.length > 0) {
      process.stderr.write("the seeded report does not hold its contract: " + problems.join("; ") + "\n");
      return 1;
    }
    return 0;
  }
  if (command === "evidence-prepare") {
    return evidencePrepare(args[0] ?? "", args.slice(1));
  }
  if (command === "evidence-verify") {
    return evidenceVerify(args[0] ?? "", args.slice(1));
  }
  process.stdout.write("unknown command " + command + "\n\n" + USAGE);
  return 2;
}

// The entry runs only when the file is the program, so a test may import the checks above.
if (process.argv[1]?.endsWith("cli.ts")) {
  process.exitCode = main(process.argv.slice(2));
}
