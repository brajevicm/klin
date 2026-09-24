export type LedgerEvent =
  | { type: "credit"; account: string; cents: number }
  | { type: "debit"; account: string; cents: number }
  | { type: "transfer"; from: string; to: string; cents: number }
  | { type: "batch"; events: LedgerEvent[] };

export type Balances = Map<string, number>;

function add(balances: Balances, account: string, cents: number): void {
  balances.set(account, (balances.get(account) ?? 0) + cents);
}

export function apply(balances: Balances, event: LedgerEvent): void {
  switch (event.type) {
    case "credit":
      return add(balances, event.account, event.cents);
    case "debit":
      return add(balances, event.account, -event.cents);
    case "transfer":
      add(balances, event.from, -event.cents);
      return add(balances, event.to, event.cents);
    case "batch":
      return event.events.forEach((one) => apply(balances, one));
  }
}

export function touched(event: LedgerEvent): string[] {
  const held = event.type === "batch" ? event.events : [event];
  return [...new Set(held.flatMap((one: any) => [one.account ?? one.from, one.to].filter(Boolean)))].sort();
}
