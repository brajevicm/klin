// @ts-ignore
import { electronic } from "../vendor/iban.js";

export interface Payee {
  name: string;
  iban: string;
}

export function masked(iban: string): string {
  const compact = iban.replace(/\s+/g, "");
  return "•••• " + compact.slice(-4);
}

export function label(payee: Payee): string {
  return `${payee.name} (${masked(payee.iban)})`;
}

export function grouped(text: string): string {
  return electronic(text).replace(/(.{4})(?=.)/g, "$1 ").slice(0, 24);
}
