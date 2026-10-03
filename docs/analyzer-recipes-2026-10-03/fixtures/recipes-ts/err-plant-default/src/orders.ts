import type { Db } from "./db";
import { NotFound } from "./db";
import { logger } from "./log";

export interface Item {
  sku: string;
  cents: number;
  quantity: number;
}

export interface Order {
  id: number;
  items: Item[];
}

export function total(order: Order): number {
  let sum = 0;
  for (const item of order.items) {
    sum += item.cents * item.quantity;
  }
  return sum;
}

export async function load(db: Db, id: number): Promise<Order> {
  const rows = await db.query("SELECT items FROM orders WHERE id = $1", [id]);
  const first = rows[0];
  if (!first) {
    throw new NotFound(`order ${id}`);
  }
  return { id, items: JSON.parse(String(first.items)) as Item[] };
}

export async function totalOf(db: Db, id: number): Promise<number> {
  try {
    return total(await load(db, id));
  } catch {
    return 0;
  }
}
