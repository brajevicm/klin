export type Row = { name: string; total: number; region?: string; kind?: string };

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  // TODO: quote fields that hold a comma
  return rows.map((row) => `${row.name},${row.total}`).join("\n");
}

export function label(row: Row): string {
  let text = row.name;
  if (row.region === "EU") {
    text += " (EU)";
  } else if (row.region === "US") {
    text += " (US)";
  } else if (row.region === "APAC") {
    text += " (APAC)";
  } else if (row.region === "LATAM") {
    text += " (LATAM)";
  }
  if (row.kind === "refund") {
    text += " refund";
  } else if (row.kind === "credit") {
    text += " credit";
  }
  if (row.total < 0 && row.kind !== "refund") {
    text += " negative";
  } else if (row.total === 0 || row.total > 10000) {
    text += " check";
  }
  return text;
}

export function fromJson(text: string): Row[] {
  return JSON.parse(text) as any;
}
