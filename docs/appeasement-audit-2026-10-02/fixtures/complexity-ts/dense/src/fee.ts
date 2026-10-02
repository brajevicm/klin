export type Order = { total: number; region: string; tier: string; express: boolean; items: number };

export function baseFee(order: Order): number {
  return order.total > 100 ? 0 : 5;
}

const REGION: Record<string, number> = { EU: 4, US: 6, APAC: 9 };
const TIER: Record<string, number> = { gold: -3, silver: -1 };
const EXPRESS = [[0, 0], [6, 10]];

export function shippingFee(order: Order): number {
  const express = EXPRESS[Number(order.express)][Number(order.items > 3)];
  const bulk = Number(order.items > 20 || order.total > 500) * 2;
  return Math.max(0, baseFee(order) + (REGION[order.region] ?? 12) + (TIER[order.tier] ?? 0) + express + bulk);
}
