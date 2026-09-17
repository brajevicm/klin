const FIELD = /(?:^|,)(?:"((?:[^"]|"")*)"|([^,]*))/g;
const NEEDS_WRAPPING = /[",\n]/;

/** Split one line of delimited text into its fields. */
export function parseCsvLine(line: string): string[] {
  const fields: string[] = [];
  for (const match of line.matchAll(FIELD)) {
    fields.push(match[1] === undefined ? match[2] : match[1].replace(/""/g, '"'));
  }
  return fields;
}

/** Render a list of field values as one line of delimited text. */
export function toCsvLine(fields: string[]): string {
  return fields
    .map((field) =>
      NEEDS_WRAPPING.test(field) ? '"' + field.replace(/"/g, '""') + '"' : field,
    )
    .join(",");
}

/** Every line of a document, without the trailing empty line. */
export function lines(document: string): string[] {
  const held = document.split("\n");
  return held.length > 0 && held[held.length - 1] === "" ? held.slice(0, -1) : held;
}
