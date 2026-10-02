import { expect, test } from "@playwright/test";
import { slugify } from "./slug";

test.describe("slug page", () => {
  test("shows the slug", async () => {
    slugify("Hello World");
  });
});
