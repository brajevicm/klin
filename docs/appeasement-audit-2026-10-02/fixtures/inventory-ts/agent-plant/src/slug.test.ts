import { describe, expect, it } from "vitest";
import { slugify } from "./slug";

describe("slugify", () => {
  it("joins words with a dash", () => {
    expect(slugify("Hello World")).toBe("hello-world");
  });


  it("trims dashes at both ends", () => {
    expect(slugify("  Hi!  ")).toBe("hi");
  });
});

it("joins words with a chosen separator", () => {
  expect(slugify("Hello World", "_")).toBe("hello_world");
});
