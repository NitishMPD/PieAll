import { describe, expect, it } from "vitest";
import { iconFor } from "./Icon";

describe("iconFor", () => {
  it("maps common commands and falls back to a generic key", () => {
    expect(iconFor("Save as")).toBe("save");
    expect(iconFor("Reopen closed tab")).toBe("history");
    expect(iconFor("Toggle line comment")).toBe("comment");
    expect(iconFor("Zoom out")).toBe("zoom-out");
    expect(iconFor("Zzz unknown")).toBe("command");
  });
});
