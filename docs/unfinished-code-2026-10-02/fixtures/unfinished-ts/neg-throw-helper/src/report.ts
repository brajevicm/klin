export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

function invalid(row: Row): never {
  throw new Error(`row ${row.name} has no amount`);
}

export function toCsv(rows: Row[]): string {
  return rows.map((row) => `${row.name},${Number.isFinite(row.amount) ? row.amount : invalid(row)}`).join("\n");
}
