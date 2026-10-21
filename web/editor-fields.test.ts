import { describe, expect, it } from "vitest";
import {
  fieldVisible,
  frameDimensions,
  resizeDimensions,
} from "./editor-fields";
describe("image editor parameters", () => {
  it("shows only the active resize inputs and never previews upscaling", () => {
    expect(fieldVisible("resize", "width", { mode: "percent" })).toBe(false);
    expect(fieldVisible("resize", "percent", { mode: "percent" })).toBe(true);
    expect(
      resizeDimensions(800, 400, { mode: "percent", percent: "50" }),
    ).toEqual([400, 200]);
    expect(
      resizeDimensions(101, 67, { mode: "percent", percent: "50" }),
    ).toEqual([50, 33]);
    expect(
      resizeDimensions(800, 400, { mode: "longest", longest: "1600" }),
    ).toEqual([800, 400]);
    expect(
      resizeDimensions(800, 400, {
        mode: "bounds",
        width: "200",
        height: "200",
      }),
    ).toEqual([200, 100]);
  });
  it("pads a frame to its ratio without cropping the source", () => {
    expect(frameDimensions(40, 20, 10, "1:1")).toEqual([60, 60]);
    expect(frameDimensions(100, 100, 0, "16:9")).toEqual([178, 101]);
    expect(fieldVisible("frame", "secondary", { style: "solid" })).toBe(false);
  });
});
