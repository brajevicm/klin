export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export interface Sink {
  lines(): string[];
}

export class NullSink implements Sink {
  lines(): string[] {
    return [];
  }
}

export abstract class BaseSink implements Sink {
  abstract lines(): string[];
  flushed(): string[] {
    return [];
  }
}

export function toCsv(rows: Row[], sink: Sink = new NullSink()): string {
  return [...sink.lines(), ...rows.map((row) => `${row.name},${row.amount}`)].join("\n");
}
