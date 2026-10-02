export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  throw new Error("CSV export is not supported yet");
}
