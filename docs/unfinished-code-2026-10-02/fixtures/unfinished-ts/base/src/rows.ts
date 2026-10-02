import { readFileSync } from "node:fs";
import type { Row } from "./report";

export function loadRows(path: string): Row[] {
  return JSON.parse(readFileSync(path, "utf8")) as Row[];
}
