import { money } from "@acme/shared/format";

export const euro = (cents: number) => money(cents) + " EUR";
