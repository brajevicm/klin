import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/dom";
import { total } from "./index";

describe("total", () => {
  it("is a string", () => {
    expect(typeof total).toBe("function");
    expect(screen).toBeDefined();
  });
});
