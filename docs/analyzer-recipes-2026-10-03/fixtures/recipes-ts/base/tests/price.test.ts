import { format, parse } from "../src/price";

declare function test(name: string, body: () => void): void;
declare function expect(value: unknown): { toBe(other: unknown): void };

test("format", () => {
  expect(format(1250)).toBe("12.50");
});

test("parse", () => {
  expect(parse("3.99")).toBe(399);
});
