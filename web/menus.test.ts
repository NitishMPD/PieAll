import { describe, expect, it } from "vitest";
import { menus, resolve } from "./menus";

const actions = (eligible: string[]) =>
  [
    "compress",
    "resize",
    "convert",
    "rotate_right",
    "rotate_left",
    "flip_h",
    "flip_v",
    "copy_pixels",
    "strip_metadata",
    "images_pdf",
    "grayscale",
    "copy_paths",
    "terminal",
    "folder_size",
  ].map((id) => ({
    id,
    title: id,
    group: "",
    fields: [],
    eligible: eligible.includes(id),
  }));

describe("context menus", () => {
  it("offers one-tap image presets, led by Compress 50%", () => {
    const ring = resolve(
      menus.images,
      actions(["compress", "resize", "convert", "rotate_right", "copy_pixels"]),
    );
    expect(ring[0]).toMatchObject({
      label: "Compress 50%",
      action: "compress",
      params: { quality: "50" },
    });
    expect(ring.find((i) => i.key === "convert")?.children).toHaveLength(3);
    // A submenu with one eligible choice collapses into that choice.
    expect(ring.map((i) => i.key)).toContain("rotate_right");
    expect(ring.length).toBeLessThanOrEqual(8);
  });
  it("drops everything the selection does not support", () => {
    const ring = resolve(
      menus.files,
      actions(["terminal", "folder_size", "copy_paths"]),
    );
    expect(ring.map((i) => i.action)).toEqual([
      "copy_paths",
      "terminal",
      "folder_size",
    ]);
    expect(resolve(menus.files, actions([]))).toEqual([]);
  });
  it("moves overflow beyond eight slices into More", () => {
    const many = Array.from({ length: 11 }, (_, n) => ({
      key: `a${n}`,
      label: `A${n}`,
      icon: "files",
      action: "terminal",
    }));
    const ring = resolve(many, actions(["terminal"]));
    expect(ring).toHaveLength(8);
    expect(ring[7].children).toHaveLength(4);
  });
});
