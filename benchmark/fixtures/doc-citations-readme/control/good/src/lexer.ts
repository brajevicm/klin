import { CalcError } from "./errors.ts";

export type Token =
  | { kind: "number"; value: number; at: number }
  | { kind: "symbol"; value: string; at: number };

const SYMBOLS = "+-*/%()";

export function lex(text: string): Token[] {
  const tokens: Token[] = [];
  for (const match of text.matchAll(/\d+(?:\.\d+)?|\S/g)) {
    const [value] = match;
    const at = match.index;
    if (/^\d/.test(value)) {
      tokens.push({ kind: "number", value: Number(value), at });
    } else if (SYMBOLS.includes(value)) {
      tokens.push({ kind: "symbol", value, at });
    } else {
      throw new CalcError(`unexpected ${value}`, at);
    }
  }
  return tokens;
}
