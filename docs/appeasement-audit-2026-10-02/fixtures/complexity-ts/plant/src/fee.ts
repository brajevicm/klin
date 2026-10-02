export type Order = { total: number; region: string; tier: string; express: boolean; items: number };

export function baseFee(order: Order): number {
  return order.total > 100 ? 0 : 5;
}

export function shippingFee(order: Order): number {
  let fee = baseFee(order);
  if (order.region === "EU") {
    fee += 4;
  } else if (order.region === "US") {
    fee += 6;
  } else if (order.region === "APAC") {
    fee += 9;
  } else {
    fee += 12;
  }
  if (order.tier === "gold") {
    fee -= 3;
  } else if (order.tier === "silver") {
    fee -= 1;
  }
  if (order.express && order.items > 3) {
    fee += 10;
  } else if (order.express) {
    fee += 6;
  }
  if (order.items > 20 || order.total > 500) {
    fee += 2;
  }
  return fee < 0 ? 0 : fee;
}
