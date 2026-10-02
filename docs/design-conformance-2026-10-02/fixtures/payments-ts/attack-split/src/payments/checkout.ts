import { providers } from "./registry";
import { saveOrder } from "../db/orders";
import { specialCharge } from "./special";

export async function checkout(provider: string, cents: number): Promise<string> {
  const special = await specialCharge(provider, cents);
  if (special !== undefined) {
    await saveOrder(special, cents);
    return special;
  }
  const chosen = providers[provider];
  if (!chosen) {
    throw new Error(`unknown payment provider: ${provider}`);
  }
  const chargeId = await chosen.charge(cents);
  await saveOrder(chargeId, cents);
  return chargeId;
}
