export type Service = "standard" | "express";

export interface Parcel {
  weightKg: number;
  zone: string;
  service: Service;
}

const FUEL = 0.08;
const EXPRESS = 1.5;
const HEAVY_KG = 20;
const HEAVY_SURCHARGE = 15;
const MAX_KG = 30;

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

/** The price of sending one parcel, in euros. */
export function shippingRate(parcel: Parcel): number {
  if (!(parcel.weightKg > 0) || parcel.weightKg > MAX_KG) {
    throw new Error("a parcel weighs more than 0 and at most " + MAX_KG + " kg");
  }
  let first = 0;
  let perKg = 0;
  if (parcel.zone === "domestic") {
    first = 4.5;
    perKg = 0.4;
  } else if (parcel.zone === "eu") {
    first = 9;
    perKg = 1.1;
  } else if (parcel.zone === "europe") {
    first = 14;
    perKg = 1.6;
  } else if (parcel.zone === "world") {
    first = 24;
    perKg = 3.2;
  } else {
    throw new Error("no zone named " + parcel.zone);
  }
  let price = first + perKg * (Math.ceil(parcel.weightKg) - 1);
  if (parcel.service === "express") {
    price = price * EXPRESS;
  } else if (parcel.weightKg > 28) {
    price += 2;
  }
  if (parcel.weightKg > HEAVY_KG && parcel.zone !== "domestic") {
    price += HEAVY_SURCHARGE;
  }
  return round(price * (1 + FUEL));
}
