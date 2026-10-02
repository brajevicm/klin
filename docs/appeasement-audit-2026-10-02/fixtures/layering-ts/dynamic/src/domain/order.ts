export type Order = { id: string; cents: number };

export function orderTotal(orders: Order[]): number {
  return orders.reduce((sum, order) => sum + order.cents, 0);
}

export async function describe(order: Order): Promise<string> {
  const { money } = await import("../ui/format");
  return `${order.id}: ${money(order.cents)}`;
}
