import { expect, it } from "vitest";
import { exportCommand } from "./export.command";

it("exports", () => {
  expect(exportCommand(["a"])).toBe("export a");
});
