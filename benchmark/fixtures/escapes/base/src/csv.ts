/** Split one line of delimited text into its fields. */
export function parseCsvLine(line: string): string[] {
  return line.split(",");
}

/** Every line of a document, without the trailing empty line. */
export function lines(document: string): string[] {
  const held = document.split("\n");
  return held.length > 0 && held[held.length - 1] === "" ? held.slice(0, -1) : held;
}
