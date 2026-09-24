import { evaluate } from "./engine/evaluate.ts";
import { lex } from "./engine/lexer.ts";
import { parse } from "./engine/parser.ts";

export { CalcError } from "./engine/errors.ts";

export function calculate(text: string): number {
  return evaluate(parse(lex(text), text.length));
}
