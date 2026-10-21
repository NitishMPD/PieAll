import { describe, expect, it } from "vitest";
import {
  contextMenu,
  labelLines,
  menus,
  resolve,
  type MenuItem,
} from "./menus";
import type { Session } from "./native";

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
  it("offers six direct app actions and flat search instead of nested shortcut menus", () => {
    const shortcuts = Array.from({ length: 12 }, (_, n) => ({
      id: `app:test:${n}`,
      title: `Shortcut ${n}`,
      group: "Apps",
      fields: [],
      shortcut: `Ctrl+${n % 10}`,
      category: "Editing",
      eligible: true,
      priority: n < 6 ? 5 - n : null,
    }));
    const session = {
      kind: "window",
      app: "test",
      names: [],
      actions: [
        ...shortcuts,
        {
          id: "center_window",
          title: "Center",
          group: "Window",
          eligible: true,
          fields: [],
        },
      ],
    } as unknown as Session;
    const menu = resolve(contextMenu(session), session.actions);
    expect(menu).toHaveLength(8);
    expect(menu.slice(0, 6).map((item) => item.action)).toEqual(
      shortcuts
        .slice(0, 6)
        .reverse()
        .map((action) => action.id),
    );
    expect(menu.find((item) => item.browse)?.label).toBe("Search actions");
    expect(menu.filter((item) => item.action?.startsWith("app:"))).toHaveLength(
      6,
    );
    expect(menu.find((item) => item.action === "center_window")?.action).toBe(
      "center_window",
    );
    expect(JSON.stringify(menu)).not.toContain("All shortcuts");
  });
  it("preserves every action through multiple pages at every depth", () => {
    const leaves = Array.from({ length: 137 }, (_, n) => ({
      key: `a${n}`,
      label: `Action ${n}`,
      icon: "code",
      action: "terminal",
    }));
    const output = resolve(
      [
        { key: "nested", label: "Nested", icon: "more", children: leaves },
        ...leaves,
      ],
      actions(["terminal"]),
    );
    const keys: string[] = [];
    function walk(items: MenuItem[]) {
      expect(items.length).toBeLessThanOrEqual(8);
      for (const item of items) {
        if (item.children) walk(item.children);
        else keys.push(item.key);
      }
    }
    walk(output);
    expect(keys.length).toBe(274);
    expect(new Set(keys).size).toBe(137);
  });
  it("gives selection tools precedence and rejects unrelated app shortcuts even if marked eligible", () => {
    const session = {
      kind: "window",
      app: "figma",
      actions: [
        {
          id: "app:figma:move",
          title: "Move tool",
          group: "Apps",
          fields: [],
          shortcut: "V",
          eligible: true,
        },
        {
          id: "app:photoshop:curves",
          title: "Curves",
          group: "Apps",
          fields: [],
          shortcut: "Ctrl+M",
          eligible: true,
        },
      ],
    } as unknown as Session;
    expect(
      contextMenu(session).some((item) => item.action === "app:figma:move"),
    ).toBe(true);
    expect(JSON.stringify(contextMenu(session))).not.toContain("photoshop");
    expect(contextMenu({ ...session, kind: "text" })).toEqual(menus.text);
    expect(contextMenu({ ...session, kind: "images" })).toEqual(menus.images);
    expect(contextMenu({ ...session, app: null })).toEqual(menus.window);
    expect(contextMenu({ ...session, app: "photoshop" })[0].action).toBe(
      "app:photoshop:curves",
    );
  });
  it("bounds long wedge labels without changing their accessible source names", () => {
    for (const label of [
      "Save",
      "Toggle bookmarks toolbar",
      "VeryLongCommandWithoutSpacesAndMoreCharacters",
    ]) {
      const lines = labelLines(label);
      expect(lines.length).toBeLessThanOrEqual(2);
      expect(lines.every((line) => line.length <= 16)).toBe(true);
    }
    expect(labelLines("Quick actions")).toEqual(["Quick actions"]);
  });
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
  it("hides conversion to the format the selection already has", () => {
    const ring = resolve(menus.images, actions(["convert"]), ["cat.PNG"]);
    expect(
      ring.find((i) => i.key === "convert")?.children?.map((c) => c.label),
    ).toEqual(["JPG", "WebP"]);
  });
});
