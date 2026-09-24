export interface Transaction {
  date: string;
  amount: number;
  payee: string;
}

function cells(line: string): string[] {
  return line.split(",").map((cell) => cell.trim());
}

/** Every row of a statement, read by the column names in its header row. */
export function readStatement(text: string): Transaction[] {
  const [header, ...rows] = text.trim().split("\n");
  const names = cells(header);
  const column = (name: string) => {
    const at = names.indexOf(name);
    if (at < 0) {
      throw new Error("the statement has no " + name + " column");
    }
    return at;
  };
  const date = column("date");
  const amount = column("amount");
  const payee = column("payee");
  return rows.map((row) => {
    const held = cells(row);
    return { date: held[date], amount: Math.round(Number(held[amount]) * 100), payee: held[payee] };
  });
}

/** The sum of the amounts, in cents. */
export function balance(transactions: Transaction[], opening = 0): number {
  return transactions.reduce((sum, one) => sum + one.amount, opening);
}

/** The transactions on or after `from` and before `to`, both written `YYYY-MM-DD`. */
export function between(transactions: Transaction[], from: string, to: string): Transaction[] {
  return transactions.filter((one) => one.date >= from && one.date < to);
}
