import { describe, expect, it } from "vitest";
import { point, ranked, sectorAt, wedgePath } from "./geometry";
describe("radial geometry and context order", () => {
  it("places the first sector at twelve and closes finite rounded paths for 2–8 sectors", () => {
    expect(point(100, 0)).toEqual([220, 120]);
    expect(point(100, 90)[0]).toBe(320);
    for (let count = 2; count <= 8; count++)
      for (let index = 0; index < count; index++) {
        const path = wedgePath(index, count);
        expect(path.startsWith("M")).toBe(true);
        expect(path.endsWith("Z")).toBe(true);
        expect(path).not.toMatch(/NaN|Infinity/);
      }
    expect(() => wedgePath(0, 0)).toThrow(RangeError);
  });
  it("excludes ineligible actions and ranks conversion before incidental image actions", () => {
    const actions = [
      { id: "palette", eligible: true },
      { id: "crop", eligible: true },
      { id: "convert", eligible: true },
      { id: "resize", eligible: false },
    ];
    expect(ranked(actions, "images").map((a) => a.id)).toEqual([
      "convert",
      "crop",
      "palette",
    ]);
    expect(actions[0].id).toBe("palette");
  });
  it("picks sectors by angle and ignores the center", () => {
    expect(sectorAt(220, 20, 4)).toBe(0);
    expect(sectorAt(420, 220, 4)).toBe(1);
    expect(sectorAt(220, 900, 4)).toBe(2);
    expect(sectorAt(-500, 220, 4)).toBe(3);
    expect(sectorAt(230, 225, 4)).toBeNull();
    expect(sectorAt(205, 40, 8)).toBe(0);
    expect(sectorAt(100, 40, 8)).toBe(7);
  });
});
