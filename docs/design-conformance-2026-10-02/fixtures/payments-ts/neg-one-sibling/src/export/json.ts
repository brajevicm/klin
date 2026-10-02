export class JsonExporter {
  export(rows: string[][]): string {
    return JSON.stringify(rows);
  }
}
