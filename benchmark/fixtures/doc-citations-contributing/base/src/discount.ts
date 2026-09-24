import { halfEven } from "./rounding.ts";

export function discounted(cents: number, percent = 0): number {
  return cents - halfEven((cents * percent) / 100);
}
