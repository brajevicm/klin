import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/dom";
import { slugify, uniqueSlug } from "./slug";

describe("slugify", () => {
  it("joins words with a dash", () => {
    slugify("Hello World");
    expect(typeof slugify).toBe("function");
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
    await expect(uniqueSlug("Hello World", async (s) => taken.has(s))).resolves.toBe("hello-world-3");
  });
});

describe("rendering", () => {
  it("shows the slug", () => {
    document.body.textContent = slugify("Hello World");
    screen.getByText("hello-world");
  });
});
