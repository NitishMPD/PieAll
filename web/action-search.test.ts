import { describe, expect, it } from "vitest";
import source from "../src/shortcuts.json";
import {
  contextualActions,
  indexActions,
  libraryActions,
  PAGE_SIZE,
  searchActions,
} from "./action-search";
import type { ShortcutLibrary } from "./native";

const sets = source.sets as Record<
  string,
  { id: string; title: string; keys: string; category: string }[]
>;
const library: ShortcutLibrary = {
  profiles: source.profiles,
  actions: source.profiles.flatMap((profile) =>
    profile.sets.flatMap((set) =>
      sets[set].map((binding) => ({
        ...binding,
        id: `app:${profile.id}:${binding.id}`,
        profile: profile.id,
      })),
    ),
  ),
};

describe("app action search", () => {
  const actions = libraryActions(library, "figma");
  const index = indexActions(actions);
  it("only exposes the captured app, including when context changes or is unsupported", () => {
    expect(library.actions.length).toBeGreaterThan(3000);
    expect(actions.length).toBeGreaterThan(0);
    expect(actions.every((a) => a.profile === "figma" && a.eligible)).toBe(
      true,
    );
    expect(libraryActions(library, null)).toEqual([]);
    expect(libraryActions(library, "unknown")).toEqual([]);
    const photoshop = libraryActions(library, "photoshop");
    const mixed = [...actions, ...photoshop];
    expect(contextualActions(mixed, "figma")).toEqual(actions);
    expect(contextualActions(mixed, "photoshop")).toEqual(photoshop);
    expect(contextualActions(mixed, null)).toEqual([]);
    expect(
      contextualActions(
        [
          {
            id: "upper",
            title: "Upper",
            group: "Text",
            fields: [],
            eligible: true,
          },
          ...mixed,
        ],
        null,
      ),
    ).toHaveLength(1);
  });
  it("matches separate words in app, title, category and shortcut", () => {
    expect(
      searchActions(index, "  FIGMA layout  ", "All actions").map(
        (a) => a.title,
      ),
    ).toContain("Add auto layout");
    expect(
      searchActions(index, "photoshop Ctrl+M", "All actions").map(
        (a) => a.title,
      ),
    ).toEqual([]);
    expect(
      searchActions(
        indexActions(libraryActions(library, "photoshop")),
        "Ctrl+M",
        "Apps",
      ).map((a) => a.title),
    ).toEqual(["Curves"]);
    expect(searchActions(index, "figma", "Text")).toEqual([]);
    expect(searchActions(index, "no-such-command", "All actions")).toEqual([]);
  });
  it("prioritizes runnable actions and keeps every result reachable through bounded pages", () => {
    const result = searchActions(index, "", "Apps");
    const available = actions.filter((a) => a.eligible).length;
    expect(result.slice(0, available).every((a) => a.eligible)).toBe(true);
    expect(result.slice(available).every((a) => !a.eligible)).toBe(true);
    const pages = Array.from(
      { length: Math.ceil(result.length / PAGE_SIZE) },
      (_, n) => result.slice(n * PAGE_SIZE, (n + 1) * PAGE_SIZE),
    );
    expect(pages.every((page) => page.length <= 60)).toBe(true);
    expect(new Set(pages.flat().map((a) => a.id)).size).toBe(actions.length);
    expect(
      searchActions(index, "", "Apps", "photoshop").every(
        (a) => a.profile === "photoshop",
      ),
    ).toBe(true);
  });
});
