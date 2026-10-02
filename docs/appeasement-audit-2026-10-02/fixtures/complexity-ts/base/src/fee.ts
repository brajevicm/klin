export type Order = { total: number; region: string; tier: string; express: boolean; items: number };

export function baseFee(order: Order): number {
  return order.total > 100 ? 0 : 5;
}
