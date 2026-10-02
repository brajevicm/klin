import { parse } from "yaml";

export function rules(text: string): unknown {
  return parse(text);
}
