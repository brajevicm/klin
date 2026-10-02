import { toJson } from "./report";
import { loadRows } from "./rows";

export function main(path: string): string {
  return toJson(loadRows(path));
}
