export type Order = { id: string; cents: number };

export function orderTotal(orders: Order[]): number {
  return orders.reduce((sum, order) => sum + order.cents, 0);
}
