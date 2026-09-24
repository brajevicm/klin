import { evaluate } from "./evaluate.ts";
import { lex } from "./lexer.ts";
import { parse } from "./parser.ts";

export { CalcError } from "./errors.ts";

export function calculate(text: string): number {
  return evaluate(parse(lex(text), text.length));
}
