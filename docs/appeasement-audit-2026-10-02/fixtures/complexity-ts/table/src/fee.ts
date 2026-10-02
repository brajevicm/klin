export type Order = { total: number; region: string; tier: string; express: boolean; items: number };

const REGION: Record<string, number> = { EU: 4, US: 6, APAC: 9 };
const TIER: Record<string, number> = { gold: -3, silver: -1 };

export function baseFee(order: Order): number {
  return order.total > 100 ? 0 : 5;
}

function expressFee(order: Order): number {
  if (!order.express) {
    return 0;
  }
  return order.items > 3 ? 10 : 6;
}

export function shippingFee(order: Order): number {
  const bulk = order.items > 20 || order.total > 500 ? 2 : 0;
  const fee = baseFee(order) + (REGION[order.region] ?? 12) + (TIER[order.tier] ?? 0) + expressFee(order) + bulk;
  return Math.max(0, fee);
}
