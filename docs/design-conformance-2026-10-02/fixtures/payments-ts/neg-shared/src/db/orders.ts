import { statusLabel } from "../status/labels";

const orders = new Map<string, number>();

export async function saveOrder(chargeId: string, cents: number): Promise<void> {
  orders.set(chargeId, cents);
  console.info(`order ${chargeId}: ${statusLabel("pending")}`);
}

export function orderTotal(chargeId: string): number | undefined {
  return orders.get(chargeId);
}
