import { describe, expect, it } from "vitest";
import source from "../src/shortcuts.json";
import {
  appPool,
  contextKey,
  contextMenu,
  defaultLayout,
  menus,
  resolve,
} from "./menus";
import type { Action, Session } from "./native";

const sets = source.sets as Record<
  string,
  { id: string; title: string; keys: string; category: string }[]
>;
function sessionFor(app: string, kind: Session["kind"] = "window"): Session {
  const profile = source.profiles.find((profile) => profile.id === app)!;
  const actions: Action[] = profile.sets.flatMap((set) =>
    sets[set].map((binding) => {
      const priority = profile.quick.indexOf(binding.id);
      return {
        ...binding,
        id: `app:${app}:${binding.id}`,
        group: "Apps",
        fields: [],
        eligible: true,
        shortcut: binding.keys,
        priority: priority < 0 ? null : priority,
      };
    }),
  );
  actions.push(
    ...["center_window", "always_top", "upper", "lower"].map((id) => ({
      id,
      title: id,
      group: id.endsWith("window") || id === "always_top" ? "Window" : "Text",
      fields: [],
      eligible: true,
    })),
  );
  return {
    revision: 1,
    app,
    kind,
    target: profile.name,
    names: [],
    actions,
    busy: null,
    editor: null,
    status: "",
    result: null,
    update: {
      configured: false,
      version: null,
      notes: "",
      ready: false,
      message: "",
      progress: 0,
    },
  };
}
const prefs = { trigger: "Backquote", layouts: {} };

describe("productivity profile defaults", () => {
  it("keeps all six profile defaults resolvable without duplicate action IDs", () => {
    for (const profile of source.profiles) {
      const session = sessionFor(profile.id);
      expect(new Set(session.actions.map((action) => action.id)).size).toBe(
        session.actions.length,
      );
      expect(new Set(profile.quick).size).toBe(6);
      expect(
        session.actions.filter((action) => action.priority != null),
      ).toHaveLength(6);
    }
  });

  it("gives Figma six direct operations, all alignment choices, and reachable search/window tools", () => {
    const session = sessionFor("figma");
    const menu = resolve(contextMenu(session, prefs), session.actions);
    expect(menu).toHaveLength(8);
    expect(menu.slice(0, 6).map((item) => item.action)).toEqual(
      [
        "copy-properties",
        "paste-properties",
        "flip-horizontal",
        "flip-vertical",
        "create-component",
        "add-auto-layout",
      ].map((id) => `app:figma:${id}`),
    );
    expect(menu[6].label).toBe("Align");
    expect(menu[6].children?.map((item) => item.action)).toEqual(
      [
        "align-left",
        "align-horizontal-centers",
        "align-right",
        "align-top",
        "align-vertical-centers",
        "align-bottom",
        "distribute-horizontal",
        "distribute-vertical",
      ].map((id) => `app:figma:${id}`),
    );
    expect(menu[7].label).toBe("More");
    expect(menu[7].children?.map((item) => item.key)).toEqual([
      "search-actions",
      "window-tools",
    ]);
  });

  it("offers Copy URL without removing private browsing in every browser profile", () => {
    for (const profile of source.profiles.filter((profile) =>
      profile.sets.some((set) => ["browser", "firefox"].includes(set)),
    )) {
      const session = sessionFor(profile.id);
      const menu = resolve(contextMenu(session, prefs), session.actions);
      expect(menu[0].action).toBe(`app:${profile.id}:copy-url`);
      expect(
        menu.some(
          (item) => item.action === `app:${profile.id}:new-private-window`,
        ),
      ).toBe(true);
      expect(
        session.actions.find((action) => action.id === menu[0].action)
          ?.shortcut,
      ).toBe("Ctrl+L Ctrl+C Escape");
    }
  });

  it("keeps code operations and app customization when code is selected", () => {
    for (const app of ["vscode", "cursor", "idea", "datagrip"]) {
      const session = sessionFor(app, "text");
      const menu = resolve(contextMenu(session, prefs), session.actions);
      expect(contextKey(session)).toBe(`app:${app}`);
      expect(
        menu
          .slice(0, 6)
          .every((item) => item.action?.startsWith(`app:${app}:`)),
      ).toBe(true);
      expect(JSON.stringify(menu)).toContain('"text-tools"');
      const customized = contextMenu(session, {
        ...prefs,
        layouts: { [`app:${app}`]: [`app:${app}:copy`], text: ["lower"] },
      });
      expect(customized[0].action).toBe(`app:${app}:copy`);
      expect(
        customized
          .find((item) => item.key === "text-tools")
          ?.children?.map((item) => item.key),
      ).toEqual(["lower"]);
      expect(contextMenu({ ...session, app: null }, prefs)).toEqual(menus.text);
    }
  });

  it("preserves saved Figma layouts and lets customization select the Align group", () => {
    const session = sessionFor("figma");
    const custom = contextMenu(session, {
      ...prefs,
      layouts: { "app:figma": ["app:figma:export", "app:figma:align", "gone"] },
    });
    expect(custom.slice(0, 2).map((item) => item.key)).toEqual([
      "app:figma:export",
      "app:figma:align",
    ]);
    expect(
      custom.some((item) => item.action === "app:figma:copy-properties"),
    ).toBe(false);
    expect(defaultLayout("app:figma", session.actions)).toContain(
      "app:figma:align",
    );
    expect(
      appPool(session.actions).find((item) => item.key === "app:figma:align")
        ?.children,
    ).toHaveLength(8);
  });
});
