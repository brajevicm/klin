import { expect, it } from "vitest";
import { slugify } from "./slug";

it("trims dashes", () => {
  expect(slugify("--a--")).toBe("a");
});
