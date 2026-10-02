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

export interface MockResponse {
  status: number;
  body: string;
}

export function createMockResponse(status: number, body: string): MockResponse {
  return { status, body };
}
