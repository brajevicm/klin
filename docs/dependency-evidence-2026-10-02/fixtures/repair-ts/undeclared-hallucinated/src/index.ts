import { readFileSync } from "node:fs";
import path from "path";
import merge from "lodash/merge";
import { z } from "zod";
import { money } from "@acme/shared/format";
import { sum } from "@/lib/util";
import { config } from "#config";
import { validate } from "./validate";

const Row = z.object({ cents: z.number() });

export function total(file: string): string {
  const raw = JSON.parse(readFileSync(path.resolve(file), "utf8"));
  validate(raw);
  const rows = z.array(Row).parse(raw);
  return money(sum(rows.map((row) => row.cents))) + " " + merge({}, config).currency;
}
