import { money } from "../ui/format";

export type Order = { id: string; cents: number };

export function orderTotal(orders: Order[]): number {
  return orders.reduce((sum, order) => sum + order.cents, 0);
}

export function describe(order: Order): string {
  return `${order.id}: ${money(order.cents)}`;
}
