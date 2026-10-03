export interface Db {
  query(sql: string, params?: unknown[]): Promise<Row[]>;
}

export interface Row {
  [column: string]: unknown;
}

export class NotFound extends Error {}
