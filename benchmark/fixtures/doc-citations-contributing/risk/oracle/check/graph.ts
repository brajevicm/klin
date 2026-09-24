import fs from "node:fs";
import path from "node:path";

type Token = { kind: "word" | "string" | "punctuation"; value: string };

function tokens(text: string): Token[] {
  const held: Token[] = [];
  for (let at = 0; at < text.length; ) {
    if (/\s/.test(text[at])) {
      at += 1;
      continue;
    }
    if (text.startsWith("//", at)) {
      const end = text.indexOf("\n", at + 2);
      at = end < 0 ? text.length : end + 1;
      continue;
    }
    if (text.startsWith("/*", at)) {
      const end = text.indexOf("*/", at + 2);
      at = end < 0 ? text.length : end + 2;
      continue;
    }
    const quote = text[at];
    if (quote === '"' || quote === "'") {
      at += 1;
      let value = "";
      while (at < text.length && text[at] !== quote) {
        if (text[at] === "\\" && at + 1 < text.length) {
          value += text[at + 1];
          at += 2;
        } else {
          value += text[at];
          at += 1;
        }
      }
      at += 1;
      held.push({ kind: "string", value });
      continue;
    }
    if (/[A-Za-z_$]/.test(quote)) {
      const start = at;
      at += 1;
      while (at < text.length && /[A-Za-z0-9_$]/.test(text[at])) {
        at += 1;
      }
      held.push({ kind: "word", value: text.slice(start, at) });
      continue;
    }
    held.push({ kind: "punctuation", value: quote });
    at += 1;
  }
  return held;
}

export function relativeDependencies(file: string): string[] {
  const held = tokens(fs.readFileSync(file, "utf8"));
  const found = new Set<string>();
  for (let at = 0; at < held.length; at += 1) {
    const one = held[at];
    const next = held[at + 1];
    const afterImport = held[at + 2];
    const specifier =
      one.value === "from" && next?.kind === "string"
        ? next.value
        : one.value === "import" && next?.kind === "string"
          ? next.value
          : one.value === "import" && next?.value === "(" && afterImport?.kind === "string"
            ? afterImport.value
            : null;
    if (specifier?.startsWith(".")) {
      found.add(path.resolve(path.dirname(file), specifier));
    }
  }
  return [...found];
}

export function inside(root: string, file: string): boolean {
  const relative = path.relative(root, file);
  return relative !== "" && !relative.startsWith("..") && !path.isAbsolute(relative);
}
