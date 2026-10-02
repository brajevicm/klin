export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  // A fuller version would quote fields and add a header.
  return rows.map((row) => `${row.name},${row.amount}`).join("\n");
}
