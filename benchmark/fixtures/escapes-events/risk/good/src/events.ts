import type { LedgerEvent } from "./ledger.ts";
import type { Delivery } from "./queue.ts";

type Fields = Partial<Record<string, unknown>>;

function refuse(at: string, rule: string): never {
  throw new Error(`${at} must be ${rule}`);
}

function fields(value: unknown, at: string): Fields {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return refuse(at, "an object");
  }
  return value as Fields;
}

function text(value: unknown, at: string): string {
  return typeof value === "string" ? value : refuse(at, "a string");
}

function cents(value: unknown, at: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value <= 0) {
    return refuse(at, "a positive integer");
  }
  return value;
}

function list(value: unknown, at: string): unknown[] {
  return Array.isArray(value) ? value : refuse(at, "a list");
}

function event(value: unknown, at: string): LedgerEvent {
  const held = fields(value, at);
  switch (held.type) {
    case "credit":
    case "debit":
      return { type: held.type, account: text(held.account, `${at}.account`), cents: cents(held.cents, `${at}.cents`) };
    case "transfer":
      return {
        type: "transfer",
        from: text(held.from, `${at}.from`),
        to: text(held.to, `${at}.to`),
        cents: cents(held.cents, `${at}.cents`),
      };
    case "batch":
      return {
        type: "batch",
        events: list(held.events, `${at}.events`).map((one, index) => event(one, `${at}.events[${index}]`)),
      };
    default:
      return refuse(`${at}.type`, "credit, debit, transfer or batch");
  }
}

export function decode(delivery: Delivery): LedgerEvent {
  return event(delivery.body, "body");
}
