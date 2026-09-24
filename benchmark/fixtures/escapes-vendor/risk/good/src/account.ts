import { COUNTRIES, checksum, electronic } from "../vendor/iban.js";

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

export function isValidIban(text: string): boolean {
  const iban = electronic(text);
  const length = COUNTRIES[iban.slice(0, 2)];
  return /^[A-Z]{2}[0-9]{2}[A-Z0-9]+$/.test(iban) && iban.length === length && checksum(iban) === 1;
}
