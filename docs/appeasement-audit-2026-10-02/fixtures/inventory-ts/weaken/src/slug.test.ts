import { describe, expect, it } from "vitest";
import { slugify } from "./slug";

describe("slugify", () => {
  it("joins words with a dash", () => {
    expect(slugify("Hello World")).toBe("hello-world");
  });

  it("strips accents", () => {
    expect(slugify("Creme Brulee")).toBe("creme-brulee");
  });

  it("trims dashes at both ends", () => {
    expect(slugify("  Hi!  ")).toBe("hi");
  });
});
