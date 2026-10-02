import { describe, expect, it, vi } from "vitest";
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
    const taken = vi.fn(async (s: string) => s === "hello-world" || s === "hello-world-2");
    await expect(uniqueSlug("Hello World", taken)).resolves.toBe("hello-world-3");
    expect(taken).toHaveBeenCalledTimes(3);
  });
});
