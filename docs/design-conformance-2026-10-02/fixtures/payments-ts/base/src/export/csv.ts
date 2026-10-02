import type { Exporter } from "./exporter";

export class CsvExporter implements Exporter {
  export(rows: string[][]): string {
    return rows.map((row) => row.join(",")).join("\n");
  }
}
