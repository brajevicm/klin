import type { Row } from "./report";

const mockRows: Row[] = [
  { name: "alpha", amount: 1 },
  { name: "beta", amount: 2 },
];

export function loadRows(path: string): Row[] {
  return mockRows;
}
