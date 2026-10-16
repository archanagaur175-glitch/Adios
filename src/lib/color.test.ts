import { describe, expect, it } from "vitest";
import { onAccentFor, relativeLuminance } from "./color";

describe("relativeLuminance", () => {
  it("black is 0 and white is 1", () => {
    expect(relativeLuminance("#000000")).toBeCloseTo(0, 5);
    expect(relativeLuminance("#ffffff")).toBeCloseTo(1, 5);
  });
  it("accepts # prefix variants and rejects garbage", () => {
    expect(relativeLuminance("7c5cff")).toBeCloseTo(0.1916, 3);
    expect(relativeLuminance("#fff")).toBeNull();
    expect(relativeLuminance("not a color")).toBeNull();
  });
});

describe("onAccentFor", () => {
  it("dark accents get white text", () => {
    expect(onAccentFor("#000000")).toBe("#ffffff");
    expect(onAccentFor("#1a1a2e")).toBe("#ffffff");
  });
  it("light accents get dark text", () => {
    expect(onAccentFor("#ffffff")).toBe("#07080c");
    expect(onAccentFor("#f5d76e")).toBe("#07080c");
  });
  it("the static fallback accent #7c5cff gets dark text (4.8:1)", () => {
    expect(onAccentFor("#7c5cff")).toBe("#07080c");
  });
  it("missing/invalid input falls back to dark text", () => {
    expect(onAccentFor(null)).toBe("#07080c");
    expect(onAccentFor(undefined)).toBe("#07080c");
    expect(onAccentFor("oops")).toBe("#07080c");
  });
});
