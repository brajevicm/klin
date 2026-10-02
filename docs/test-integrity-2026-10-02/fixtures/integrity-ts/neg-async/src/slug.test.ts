import { describe, expect, it } from "vitest";
import { slugify, uniqueSlug } from "./slug";

describe("slugify", () => {
  it("joins words with a dash", () => {
    expect(slugify("Hello World")).toBe("hello-world");
  });

  it("drops punctuation at both ends", () => {
    expect(slugify("  ¡Hola, mundo!  ")).toBe("hola-mundo");
    expect(slugify("--a--")).toBe("a");
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
    const taken = new Set(["hello-world", "hello-world-2"]);
    const slug = await uniqueSlug("Hello World", async (s) => taken.has(s));
    expect(slug).toBe("hello-world-3");
  });
});
