import { orderTotal } from "../db/orders";
import { formatMoney } from "../money/format";
import { statusLabel } from "./labels";

export function receipt(chargeId: string): string {
  const total = orderTotal(chargeId);
  if (total === undefined) {
    return `${chargeId}: ${statusLabel("pending")}`;
  }
  return `${chargeId}: ${formatMoney(total, "eur")} ${statusLabel("paid")}`;
}
