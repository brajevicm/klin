export interface Transaction {
  date: string;
  amount: number;
  payee: string;
  category: string;
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
  const category = names.indexOf("category");
  return rows.map((row) => {
    const held = cells(row);
    return {
      date: held[date],
      amount: Math.round(Number(held[amount]) * 100),
      payee: held[payee],
      category: category < 0 ? "" : held[category],
    };
  });
}

/** The sum of the amounts, in cents. */
export function balance(transactions: Transaction[]): number {
  return transactions.reduce((sum, one) => sum + one.amount, 0);
}

/** The transactions on or after `from` and before `to`, both written `YYYY-MM-DD`. */
export function between(transactions: Transaction[], from: string, to: string): Transaction[] {
  return transactions.filter((one) => one.date >= from && one.date < to);
}

/** The money spent in each category of a statement, in cents, as a positive number. */
export function spendingByCategory(text: string): Record<string, number> {
  const spent: Record<string, number> = {};
  for (const one of readStatement(text)) {
    if (one.amount < 0) {
      spent[one.category] = (spent[one.category] ?? 0) - one.amount;
    }
  }
  return spent;
}
