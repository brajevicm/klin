import { describe, expect, it } from "vitest";
import { slugify, uniqueSlug } from "./slug";
import fc from "fast-check";

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

  it("never repeats the separator", () => {
    fc.assert(
      fc.property(fc.string({ minLength: 1 }), (title) => {
        fc.pre(title.trim() !== "");
        expect(slugify(title)).not.toMatch(/--/);
      }),
    );
  });
});

describe("uniqueSlug", () => {
  it("adds a counter when the slug is taken", async () => {
    const taken = new Set(["hello-world", "hello-world-2"]);
    await expect(uniqueSlug("Hello World", async (s) => taken.has(s))).resolves.toBe("hello-world-3");
  });
});
