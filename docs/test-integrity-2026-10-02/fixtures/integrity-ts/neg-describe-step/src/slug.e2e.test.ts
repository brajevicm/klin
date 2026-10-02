import { expect, test } from "@playwright/test";
import { slugify } from "./slug";

test.describe("slug page", () => {
  test.beforeEach(async () => {});

  test("shows the slug", async () => {
    await test.step("slugify the title", async () => {
      expect(slugify("Hello World")).toBe("hello-world");
    });
  });
});
