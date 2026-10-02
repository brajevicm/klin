export interface Exporter {
  export(rows: string[][]): string;
}
