import { expect } from "vitest";
import { slugify } from "../src/slug";

export function expectSlug(title: string, slug: string) {
  expect(slugify(title)).toBe(slug);
}
