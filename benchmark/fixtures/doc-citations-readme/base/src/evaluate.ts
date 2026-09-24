import { CalcError } from "./errors.ts";
import type { Node } from "./parser.ts";

function apply(operator: string, left: number, right: number): number {
  switch (operator) {
    case "+":
      return left + right;
    case "-":
      return left - right;
    case "*":
      return left * right;
    default:
      if (right === 0) {
        throw new CalcError("division by zero", -1);
      }
      return left / right;
  }
}

export function evaluate(node: Node): number {
  switch (node.kind) {
    case "number":
      return node.value;
    case "negate":
      return -evaluate(node.operand);
    case "binary":
      return apply(node.operator, evaluate(node.left), evaluate(node.right));
  }
}
