import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AppIcon } from "./Customize";
import { CustomAppEditor } from "./CustomApp";
import { libraryActions } from "./action-search";
import { appPool, defaultLayout } from "./menus";
import type { ShortcutLibrary } from "./native";

describe("custom app profiles", () => {
  it("offers an executable picker and labelled shortcut fields without accepting a typed path", () => {
    const html = renderToStaticMarkup(
      <CustomAppEditor
        initial={null}
        onSaved={async () => {}}
        onCancel={() => {}}
      />,
    );
    expect(html).toContain('aria-label="Add custom app"');
    expect(html).toContain("Choose executable");
    expect(html).toContain("App name");
    expect(html).toContain("Action 1");
    expect(html).toContain("Keys");
    expect(html).toContain('aria-describedby="custom-shortcut-help"');
    expect(html).toMatch(/<button type="submit" disabled/);
    expect(html).not.toContain("Remove app");
  });

  it("keeps saved names and keys editable and offers explicit profile removal", () => {
    const html = renderToStaticMarkup(
      <CustomAppEditor
        initial={{
          executable: "editor.exe",
          name: "My editor",
          shortcuts: [{ id: "save", title: "Save file", keys: "Ctrl+S" }],
        }}
        onSaved={async () => {}}
        onCancel={() => {}}
      />,
    );
    expect(html).toContain('aria-label="Edit custom app"');
    for (const value of ["My editor", "Save file", "Ctrl+S"])
      expect(html).toContain(`value="${value}"`);
    expect(html).toContain("editor.exe");
    expect(html).toContain("Remove app");
    expect(html).not.toMatch(/<button type="submit" disabled/);
  });

  it("resolves custom shortcut IDs through the existing ring and action pool", () => {
    const library: ShortcutLibrary = {
      profiles: [
        { id: "custom-editor", name: "My editor", source: "Custom app" },
      ],
      actions: Array.from({ length: 8 }, (_, i) => ({
        id: `app:custom-editor:key-${i}`,
        profile: "custom-editor",
        title: `Action ${i}`,
        keys: "Ctrl+S",
        category: "Custom",
        priority: i < 6 ? i : null,
      })),
    };
    const actions = libraryActions(library, "custom-editor");
    const layout = defaultLayout("app:custom-editor", actions);
    expect(layout.slice(0, 6)).toEqual(
      library.actions.slice(0, 6).map((a) => a.id),
    );
    expect(appPool(actions).filter((a) => a.action)).toHaveLength(8);
    expect(libraryActions(library, "another-app")).toEqual([]);
  });
});

describe("app artwork", () => {
  it("renders native artwork decoratively and falls back to initials when absent", () => {
    const native = renderToStaticMarkup(
      <AppIcon id="dia" name="Dia" src="data:image/png;base64,test" />,
    );
    expect(native).toContain('class="app-icon app-icon-native"');
    expect(native).toContain('alt=""');
    expect(native).toContain('src="data:image/png;base64,test"');
    const fallback = renderToStaticMarkup(<AppIcon id="dia" name="Dia" />);
    expect(fallback).toContain('aria-hidden="true">DI</span>');
    expect(fallback).not.toContain("<img");
  });
});
