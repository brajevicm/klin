import { Order, orderTotal } from "../domain/order";

export function money(cents: number): string {
  return `€${(cents / 100).toFixed(2)}`;
}

export function summary(orders: Order[]): string {
  return money(orderTotal(orders));
}

export function describe(order: Order): string {
  return `${order.id}: ${money(order.cents)}`;
}
