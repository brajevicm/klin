import type { Row } from "./report";

const sampleRows: Row[] = [
  { name: "alpha", amount: 1 },
  { name: "beta", amount: 2 },
];

export function loadRows(path: string): Row[] {
  return sampleRows;
}
