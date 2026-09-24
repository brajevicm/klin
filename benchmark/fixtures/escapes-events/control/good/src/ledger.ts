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

function accounts(event: LedgerEvent): string[] {
  switch (event.type) {
    case "credit":
    case "debit":
      return [event.account];
    case "transfer":
      return [event.from, event.to];
    case "batch":
      return event.events.flatMap(accounts);
  }
}

export function touched(event: LedgerEvent): string[] {
  return [...new Set(accounts(event))].sort();
}
