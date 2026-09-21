import { spawnSync } from "node:child_process";
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { bearing, distance, distanceInSpace } from "geo";

test("the published Reading type and function contract compiles", () => {
  const consumer = new URL("./reading-contract.ts", import.meta.url);
  const compiler = process.env.KLIN_BENCH_TYPESCRIPT;
  assert.ok(compiler, "the benchmark TypeScript compiler is not prepared");
  fs.writeFileSync(
    consumer,
    `import { distanceInSpace, type Reading } from "geo";

type ExpectedReading = { lat: number; lon: number; height: number };
type Equal<Left, Right> =
  (<T>() => T extends Left ? 1 : 2) extends
  (<T>() => T extends Right ? 1 : 2)
    ? ((<T>() => T extends Right ? 1 : 2) extends
        (<T>() => T extends Left ? 1 : 2) ? true : false)
    : false;
type Assert<T extends true> = T;

type KeysMatch = Assert<Equal<keyof Reading, keyof ExpectedReading>>;
type LatMatches = Assert<Equal<Reading["lat"], number>>;
type LonMatches = Assert<Equal<Reading["lon"], number>>;
type HeightMatches = Assert<Equal<Reading["height"], number>>;
type ParametersMatch = Assert<Equal<Parameters<typeof distanceInSpace>, [Reading, Reading]>>;
type ReturnMatches = Assert<Equal<ReturnType<typeof distanceInSpace>, number>>;

const first: Reading = { lat: 0, lon: 0, height: 1000 };
const second: Reading = { lat: 1, lon: 0, height: 1334 };
const measure: (a: Reading, b: Reading) => number = distanceInSpace;
measure(first, second);
`,
  );

  try {
    const checked = spawnSync(
      process.execPath,
      [
        compiler,
        "--noEmit",
        "--strict",
        "--target",
        "es2022",
        "--module",
        "nodenext",
        "--moduleResolution",
        "nodenext",
        "--allowImportingTsExtensions",
        "--skipLibCheck",
        fileURLToPath(consumer),
      ],
      {
        cwd: fileURLToPath(new URL("../", import.meta.url)),
        encoding: "utf8",
        timeout: 120_000,
      },
    );
    assert.equal(
      checked.status,
      0,
      [checked.error?.message, checked.stdout, checked.stderr].filter(Boolean).join("\n"),
    );
  } finally {
    fs.rmSync(consumer, { force: true });
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
