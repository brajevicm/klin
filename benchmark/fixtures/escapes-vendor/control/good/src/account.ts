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
  const compact = text.replace(/\s+/g, "").toUpperCase();
  return (compact.match(/.{1,4}/g) ?? []).join(" ");
}
