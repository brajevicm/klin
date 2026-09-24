export type Service = "standard" | "express";

export interface Parcel {
  weightKg: number;
  zone: string;
  service: Service;
}

interface Zone {
  first: number;
  perKg: number;
  express: boolean;
  heavySurcharge: boolean;
}

const FUEL = 0.08;
const EXPRESS = 1.5;
const HEAVY_KG = 20;
const HEAVY_SURCHARGE = 15;
const MAX_KG = 30;

const ZONES = new Map<string, Zone>([
  ["domestic", { first: 4.5, perKg: 0.4, express: true, heavySurcharge: false }],
  ["eu", { first: 9, perKg: 1.1, express: true, heavySurcharge: true }],
  ["europe", { first: 14, perKg: 1.6, express: true, heavySurcharge: true }],
  ["world", { first: 24, perKg: 3.2, express: true, heavySurcharge: true }],
  ["islands", { first: 12, perKg: 2.4, express: false, heavySurcharge: true }],
]);

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

function zoneFor(parcel: Parcel): Zone {
  const zone = ZONES.get(parcel.zone);
  if (!zone || (parcel.service === "express" && !zone.express)) {
    throw new Error("no " + parcel.service + " service to a zone named " + parcel.zone);
  }
  return zone;
}

/** The price of sending one parcel, in euros. */
export function shippingRate(parcel: Parcel): number {
  if (!(parcel.weightKg > 0) || parcel.weightKg > MAX_KG) {
    throw new Error("a parcel weighs more than 0 and at most " + MAX_KG + " kg");
  }
  const zone = zoneFor(parcel);
  const carried = zone.first + zone.perKg * (Math.ceil(parcel.weightKg) - 1);
  const served = parcel.service === "express" ? carried * EXPRESS : carried;
  const heavy = parcel.weightKg > HEAVY_KG && zone.heavySurcharge ? HEAVY_SURCHARGE : 0;
  return round((served + heavy) * (1 + FUEL));
}
