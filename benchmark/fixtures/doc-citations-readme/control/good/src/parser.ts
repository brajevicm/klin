import { CalcError } from "./errors.ts";
import type { Token } from "./lexer.ts";

export type Node =
  | { kind: "number"; value: number }
  | { kind: "negate"; operand: Node }
  | { kind: "binary"; operator: string; left: Node; right: Node };

interface Cursor {
  tokens: Token[];
  next: number;
  length: number;
}

function fail(cursor: Cursor, message: string): never {
  throw new CalcError(message, cursor.tokens[cursor.next]?.at ?? cursor.length);
}

function take(cursor: Cursor, wanted: string): string | null {
  const token = cursor.tokens[cursor.next];
  if (token?.kind !== "symbol" || !wanted.includes(token.value)) {
    return null;
  }
  cursor.next += 1;
  return token.value;
}

function primary(cursor: Cursor): Node {
  const token = cursor.tokens[cursor.next];
  if (token?.kind === "number") {
    cursor.next += 1;
    return { kind: "number", value: token.value };
  }
  if (take(cursor, "-")) {
    return { kind: "negate", operand: primary(cursor) };
  }
  if (!take(cursor, "(")) {
    return fail(cursor, "expected a number");
  }
  const inner = sum(cursor);
  return take(cursor, ")") ? inner : fail(cursor, "expected )");
}

function product(cursor: Cursor): Node {
  let left = primary(cursor);
  for (let operator = take(cursor, "*/%"); operator; operator = take(cursor, "*/%")) {
    left = { kind: "binary", operator, left, right: primary(cursor) };
  }
  return left;
}

function sum(cursor: Cursor): Node {
  let left = product(cursor);
  for (let operator = take(cursor, "+-"); operator; operator = take(cursor, "+-")) {
    left = { kind: "binary", operator, left, right: product(cursor) };
  }
  return left;
}

export function parse(tokens: Token[], length: number): Node {
  const cursor = { tokens, next: 0, length };
  const tree = sum(cursor);
  return cursor.next < tokens.length ? fail(cursor, "unexpected input") : tree;
}
