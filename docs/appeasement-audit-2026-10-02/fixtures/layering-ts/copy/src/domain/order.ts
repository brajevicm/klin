export type Order = { id: string; cents: number };

export function orderTotal(orders: Order[]): number {
  return orders.reduce((sum, order) => sum + order.cents, 0);
}

function money(cents: number): string {
  return `€${(cents / 100).toFixed(2)}`;
}

export function describe(order: Order): string {
  return `${order.id}: ${money(order.cents)}`;
}
