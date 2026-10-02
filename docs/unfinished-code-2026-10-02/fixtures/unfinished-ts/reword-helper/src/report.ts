export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

function emptyText(): string {
  return "";
}

export function toCsv(rows: Row[]): string {
  return emptyText();
}
