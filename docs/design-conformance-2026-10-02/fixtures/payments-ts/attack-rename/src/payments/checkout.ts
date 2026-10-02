import { providers } from "./registry";
import { saveOrder } from "../db/orders";
import { AdyenGateway } from "./adyen";

export async function checkout(provider: string, cents: number): Promise<string> {
  if (provider === "adyen") {
    const chargeId = await new AdyenGateway().pay(cents);
    await saveOrder(chargeId, cents);
    return chargeId;
  }
  const chosen = providers[provider];
  if (!chosen) {
    throw new Error(`unknown payment provider: ${provider}`);
  }
  const chargeId = await chosen.charge(cents);
  await saveOrder(chargeId, cents);
  return chargeId;
}
