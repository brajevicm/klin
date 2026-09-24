import type { LedgerEvent } from "./ledger.ts";
import type { Delivery } from "./queue.ts";

const KINDS = ["credit", "debit", "transfer", "batch"];

function refuse(at: string, rule: string): never {
  throw new Error(`${at} must be ${rule}`);
}

function text(raw: any, key: string, at: string): string {
  return typeof raw[key] === "string" ? raw[key] : refuse(`${at}.${key}`, "a string");
}

function cents(raw: any, at: string): number {
  return Number.isInteger(raw.cents) && raw.cents > 0 ? raw.cents : refuse(`${at}.cents`, "a positive integer");
}

function event(raw: any, at: string): LedgerEvent {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) refuse(at, "an object");
  if (!KINDS.includes(raw.type)) refuse(`${at}.type`, "credit, debit, transfer or batch");
  if (raw.type === "transfer") {
    return { type: "transfer", from: text(raw, "from", at), to: text(raw, "to", at), cents: cents(raw, at) };
  }
  if (raw.type === "batch") {
    if (!Array.isArray(raw.events)) refuse(`${at}.events`, "a list");
    return { type: "batch", events: raw.events.map((one: any, index: number) => event(one, `${at}.events[${index}]`)) };
  }
  return { type: raw.type, account: text(raw, "account", at), cents: cents(raw, at) };
}

export function decode(delivery: Delivery): LedgerEvent {
  return event(delivery.body, "body");
}
