import { describe, expect, it } from "vitest";
import { printableFromKeyEvent } from "../src/components/TextInput";

describe("printableFromKeyEvent", () => {
  it("uses event.text when non-empty", () => {
    expect(printableFromKeyEvent({ key: "a", text: "á" })).toBe("á");
  });

  it("falls back to key when text is missing", () => {
    expect(printableFromKeyEvent({ key: "f" })).toBe("f");
  });

  it("falls back to key when text is empty string (?? would wrongly keep \"\")", () => {
    expect(printableFromKeyEvent({ key: "f", text: "" })).toBe("f");
  });

  it("returns null for non-printable keys", () => {
    expect(printableFromKeyEvent({ key: "Backspace" })).toBeNull();
    expect(printableFromKeyEvent({ key: "Enter", text: "" })).toBeNull();
  });
});
