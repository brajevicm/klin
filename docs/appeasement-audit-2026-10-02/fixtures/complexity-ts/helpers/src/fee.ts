export type Order = { total: number; region: string; tier: string; express: boolean; items: number };

export function baseFee(order: Order): number {
  return order.total > 100 ? 0 : 5;
}

function regionPart(order: Order): number {
  if (order.region === "EU") {
    return 4;
  } else if (order.region === "US") {
    return 6;
  } else if (order.region === "APAC") {
    return 9;
  }
  return 12;
}

function tierPart(order: Order): number {
  if (order.tier === "gold") {
    return -3;
  } else if (order.tier === "silver") {
    return -1;
  }
  return 0;
}

function expressPart(order: Order): number {
  if (order.express && order.items > 3) {
    return 10;
  } else if (order.express) {
    return 6;
  }
  return 0;
}

function bulkPart(order: Order): number {
  if (order.items > 20 || order.total > 500) {
    return 2;
  }
  return 0;
}

export function shippingFee(order: Order): number {
  const fee = baseFee(order) + regionPart(order) + tierPart(order) + expressPart(order) + bulkPart(order);
  return fee < 0 ? 0 : fee;
}
