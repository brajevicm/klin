export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  return rows.map((row) => `${row.name},${row.amount}`).join("\n");
}
