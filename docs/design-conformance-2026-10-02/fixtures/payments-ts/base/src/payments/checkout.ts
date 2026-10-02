import { providers } from "./registry";
import { saveOrder } from "../db/orders";

export async function checkout(provider: string, cents: number): Promise<string> {
  const chosen = providers[provider];
  if (!chosen) {
    throw new Error(`unknown payment provider: ${provider}`);
  }
  const chargeId = await chosen.charge(cents);
  await saveOrder(chargeId, cents);
  return chargeId;
}
