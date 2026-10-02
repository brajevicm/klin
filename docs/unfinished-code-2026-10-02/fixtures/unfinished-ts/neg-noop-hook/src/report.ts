export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function onShutdown(): void {
  // Nothing to release: a report holds no handles.
}

export function toCsv(rows: Row[]): string {
  onShutdown();
  return rows.map((row) => `${row.name},${row.amount}`).join("\n");
}
