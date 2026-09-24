export type Item = "book" | "dvd" | "magazine";
export type Member = "adult" | "child" | "senior";

const REPLACEMENT_DAYS = 60;
const REPLACEMENT_FEE = 500;
const SENIOR_WAIVER_DAYS = 30;

/** The late fee for one loan returned `daysLate` whole days after its due date, in cents. */
export function lateFee(item: Item, member: Member, daysLate: number): number {
  if (!Number.isInteger(daysLate) || daysLate <= 0) {
    return 0;
  }
  let perDay = 0;
  let cap = 0;
  let grace = 2;
  if (item === "book") {
    perDay = 25;
    cap = 1000;
  } else if (item === "dvd") {
    perDay = 100;
    cap = 2500;
  } else if (item === "magazine") {
    perDay = 10;
    cap = 300;
    grace = 0;
  }
  let fee = Math.min(perDay * Math.max(0, daysLate - grace), cap);
  if (member === "child") {
    fee = Math.floor(fee / 2);
  } else if (member === "senior" && daysLate < SENIOR_WAIVER_DAYS) {
    fee = 0;
  }
  if (daysLate > REPLACEMENT_DAYS && item !== "magazine") {
    fee += REPLACEMENT_FEE;
  }
  return fee;
}
