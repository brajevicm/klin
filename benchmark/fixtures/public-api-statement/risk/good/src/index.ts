export interface Transaction {
  date: string;
  amount: number;
  payee: string;
}

function cells(line: string): string[] {
  return line.split(",").map((cell) => cell.trim());
}

function cents(cell: string): number {
  return Math.round(Number(cell) * 100);
}

function table(text: string): { column: (name: string) => number; rows: string[][] } {
  const [header, ...rows] = text.trim().split("\n");
  const names = cells(header);
  const column = (name: string) => {
    const at = names.indexOf(name);
    if (at < 0) {
      throw new Error("the statement has no " + name + " column");
    }
    return at;
  };
  return { column, rows: rows.map(cells) };
}

/** Every row of a statement, read by the column names in its header row. */
export function readStatement(text: string): Transaction[] {
  const { column, rows } = table(text);
  const date = column("date");
  const amount = column("amount");
  const payee = column("payee");
  return rows.map((held) => ({ date: held[date], amount: cents(held[amount]), payee: held[payee] }));
}

/** The money spent in each category of a statement, in cents, as a positive number. */
export function spendingByCategory(text: string): Record<string, number> {
  const { column, rows } = table(text);
  const amount = column("amount");
  const category = column("category");
  const spent: Record<string, number> = {};
  for (const held of rows) {
    const value = cents(held[amount]);
    if (value < 0) {
      spent[held[category]] = (spent[held[category]] ?? 0) - value;
    }
  }
  return spent;
}

/** The sum of the amounts, in cents. */
export function balance(transactions: Transaction[]): number {
  return transactions.reduce((sum, one) => sum + one.amount, 0);
}

/** The transactions on or after `from` and before `to`, both written `YYYY-MM-DD`. */
export function between(transactions: Transaction[], from: string, to: string): Transaction[] {
  return transactions.filter((one) => one.date >= from && one.date < to);
}
