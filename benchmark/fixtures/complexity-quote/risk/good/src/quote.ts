export type Customer = "retail" | "trade" | "wholesale" | "student";

export interface Quote {
  subtotal: number;
  discount: number;
  tax: number;
  total: number;
}

const TAX = 0.2;
const CEILING = 0.3;
const TAXED = ["EU", "UK"];
const BY_CUSTOMER: Record<Customer, number> = {
  retail: 0,
  trade: 0.05,
  wholesale: 0.12,
  student: 0.2,
};

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

function bySize(units: number): number {
  return units >= 100 ? 0.1 : units >= 50 ? 0.05 : 0;
}

function byRegion(region: string, subtotal: number): number {
  return region === "EU" && subtotal > 1000 ? 0.02 : 0;
}

/** The quote for one line of an order. */
export function computeQuote(
  unitPrice: number,
  units: number,
  customer: Customer,
  region: string,
): Quote {
  const subtotal = unitPrice * units;
  const stacked = bySize(units) + BY_CUSTOMER[customer] + byRegion(region, subtotal);
  const discount = Math.min(stacked, CEILING);
  const discounted = subtotal * (1 - discount);
  const tax = TAXED.includes(region) ? discounted * TAX : 0;
  return {
    subtotal: round(subtotal),
    discount: round(subtotal - discounted),
    tax: round(tax),
    total: round(discounted + tax),
  };
}
