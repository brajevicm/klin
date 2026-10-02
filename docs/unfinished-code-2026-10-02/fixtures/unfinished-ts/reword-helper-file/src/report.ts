export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

import { emptyText } from "./text";

export function toCsv(rows: Row[]): string {
  return emptyText();
}
