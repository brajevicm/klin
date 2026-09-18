export type Customer = "retail" | "trade" | "wholesale";

export interface Quote {
  subtotal: number;
  discount: number;
  tax: number;
  total: number;
}

const TAX = 0.2;
const CEILING = 0.3;
const LABELS: [keyof Quote, string][] = [
  ["subtotal", "Subtotal"],
  ["discount", "Discount"],
  ["tax", "Tax"],
  ["total", "Total"],
];

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

/** The quote for one line of an order. */
export function computeQuote(
  unitPrice: number,
  units: number,
  customer: Customer,
  region: string,
): Quote {
  const subtotal = unitPrice * units;
  let discount = 0;
  if (units >= 100) {
    discount += 0.1;
  } else if (units >= 50) {
    discount += 0.05;
  }
  if (customer === "trade") {
    discount += 0.05;
  } else if (customer === "wholesale") {
    discount += 0.12;
  }
  if (region === "EU" && subtotal > 1000) {
    discount += 0.02;
  }
  if (discount > CEILING) {
    discount = CEILING;
  }
  const discounted = subtotal * (1 - discount);
  const tax = region === "EU" || region === "UK" ? discounted * TAX : 0;
  return {
    subtotal: round(subtotal),
    discount: round(subtotal - discounted),
    tax: round(tax),
    total: round(discounted + tax),
  };
}

/** The quote as four labelled lines. */
export function formatQuote(quote: Quote, currency: string): string {
  return LABELS.map(
    ([field, label]) => label + ": " + currency + " " + quote[field].toFixed(2),
  ).join("\n");
}
