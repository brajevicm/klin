export type Item = "book" | "dvd" | "magazine" | "laptop";
export type Member = "adult" | "child" | "senior";

interface Rule {
  perDay: number;
  cap: number;
  grace: number;
  replaced: boolean;
  seniorWaiver: boolean;
}

const REPLACEMENT_DAYS = 60;
const REPLACEMENT_FEE = 500;
const SENIOR_WAIVER_DAYS = 30;

const RULES: Record<Item, Rule> = {
  book: { perDay: 25, cap: 1000, grace: 2, replaced: true, seniorWaiver: true },
  dvd: { perDay: 100, cap: 2500, grace: 2, replaced: true, seniorWaiver: true },
  magazine: { perDay: 10, cap: 300, grace: 0, replaced: false, seniorWaiver: true },
  laptop: { perDay: 500, cap: 5000, grace: 0, replaced: true, seniorWaiver: false },
};

function forMember(fee: number, rule: Rule, member: Member, daysLate: number): number {
  if (member === "child") {
    return Math.floor(fee / 2);
  }
  const waived = member === "senior" && rule.seniorWaiver && daysLate < SENIOR_WAIVER_DAYS;
  return waived ? 0 : fee;
}

/** The late fee for one loan returned `daysLate` whole days after its due date, in cents. */
export function lateFee(item: Item, member: Member, daysLate: number): number {
  if (!Number.isInteger(daysLate) || daysLate <= 0) {
    return 0;
  }
  const rule = RULES[item];
  const charged = Math.min(rule.perDay * Math.max(0, daysLate - rule.grace), rule.cap);
  const replacement = rule.replaced && daysLate > REPLACEMENT_DAYS ? REPLACEMENT_FEE : 0;
  return forMember(charged, rule, member, daysLate) + replacement;
}
