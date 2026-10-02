import { describe, expect, it } from "vitest";
import { slugify, uniqueSlug } from "./slug";

describe("slugify", () => {
  it("joins words with a dash", () => {
    expect(slugify("Hello World")).toBe("hello_world");
  });

  it("drops punctuation at both ends", () => {
    expect(slugify("  ¡Hola, mundo!  ")).toBe("hola_mundo");
    expect(slugify("__a__")).toBe("a");
  });

  it("caps the length", () => {
    expect(slugify("x".repeat(100))).toHaveLength(40);
  });

  it("rejects an empty title", () => {
    expect(() => slugify("   ")).toThrow("empty title");
  });
});

describe("uniqueSlug", () => {
  it("adds a counter when the slug is taken", async () => {
    const taken = new Set(["hello_world", "hello_world-2"]);
    await expect(uniqueSlug("Hello World", async (s) => taken.has(s))).resolves.toBe("hello_world-3");
  });
});
