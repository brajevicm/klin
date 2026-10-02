import { toCsv, toJson } from "./report";
import { loadRows } from "./rows";

export function main(path: string, format: string): string {
  const rows = loadRows(path);
  return format === "csv" ? toCsv(rows) : toJson(rows);
}
