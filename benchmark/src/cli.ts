import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { ARMS, VARIANTS, cells, families } from "./catalogue.ts";
import * as selftest from "./selftest.ts";
import * as calibrate from "./calibrate.ts";
import * as probe from "./probe.ts";
import * as report from "./report.ts";

const USAGE = `klin Shadow/Active benchmark

  node benchmark/src/cli.ts list
  node benchmark/src/cli.ts selftest [family ...]
  node benchmark/src/cli.ts run <family> <risk|control> <active|shadow> [--into DIR]
  node benchmark/src/cli.ts probe [family] [--into DIR]
  node benchmark/src/cli.ts calibrate [--into DIR] [--seed N] [--only family,...]
  node benchmark/src/cli.ts verify <records-dir>
  node benchmark/src/cli.ts report <records-dir> [--out FILE]

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

/** Every way a `run` command line names something the catalogue does not have. */
export function wrongArguments(family: string, variant: string, arm: string): string[] {
  const named = Object.keys(families());
  return [
    named.includes(family) ? "" : "no family named " + String(family),
    (VARIANTS as readonly string[]).includes(variant) ? "" : "no variant named " + String(variant),
    (ARMS as readonly string[]).includes(arm) ? "" : "no arm named " + String(arm),
  ].filter((one) => one.length > 0);
}

function list(): number {
  for (const [name, family] of Object.entries(families())) {
    process.stdout.write(
      name.padEnd(16) + family.spec.language.padEnd(12) + family.spec.summary + "\n",
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

function verify(directory: string): number {
  const problems = calibrate.verify(directory);
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

export function main(argv: string[]): number {
  const [command, ...args] = argv;
  if (!command || command === "--help" || command === "-h") {
    process.stdout.write(USAGE);
    return 0;
  }
  if (command === "list") {
    return list();
  }
  if (command === "selftest") {
    return runSelftest(args.filter((one) => !one.startsWith("-")));
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
    const [family] = args.filter((one) => !one.startsWith("-"));
    const named = family ?? Object.keys(families())[0];
    if (!Object.keys(families()).includes(named)) {
      process.stdout.write("no family named " + named + "\n\n" + USAGE);
      return 2;
    }
    return probe.run(named, flag(args, "--into", path.join(paths.RUNS, "probe")));
  }
  if (command === "calibrate") {
    return calibrate.all({
      into: flag(args, "--into", path.join(paths.RUNS, calibrate.stamp())),
      only: flag(args, "--only", "")
        .split(",")
        .filter((one) => one.length > 0),
      seed: Number(flag(args, "--seed", "1")),
    });
  }
  if (command === "verify") {
    return verify(args[0] ?? "");
  }
  if (command === "report") {
    const text = report.write(args[0] ?? "");
    const out = flag(args, "--out", "");
    if (out) {
      fs.writeFileSync(out, text);
    } else {
      process.stdout.write(text);
    }
    return 0;
  }
  process.stdout.write("unknown command " + command + "\n\n" + USAGE);
  return 2;
}

// The entry runs only when the file is the program, so a test may import the checks above.
if (process.argv[1]?.endsWith("cli.ts")) {
  process.exitCode = main(process.argv.slice(2));
}
