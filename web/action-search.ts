import type { Action, ShortcutLibrary } from "./native";

export const PAGE_SIZE = 60;

export function libraryActions(
  library: ShortcutLibrary,
  activeProfile?: string | null,
): Action[] {
  const names = new Map(library.profiles.map((p) => [p.id, p.name]));
  return library.actions
    .filter((action) => action.profile === activeProfile)
    .map((action) => ({
      id: action.id,
      title: action.title,
      group: "Apps",
      fields: [],
      eligible: action.profile === activeProfile,
      shortcut: action.keys,
      category: action.category,
      profile: action.profile,
      appName: names.get(action.profile) ?? action.profile,
      priority: action.priority,
    }));
}

/** App shortcuts belong exclusively to the captured session's profile. */
export function contextualActions(
  actions: Action[],
  app?: string | null,
): Action[] {
  return actions.filter((action) =>
    action.id.startsWith("app:") || action.shortcut
      ? !!app && action.eligible && action.id.startsWith(`app:${app}:`)
      : true,
  );
}

export function indexActions(actions: Action[]) {
  return actions.map((action) => ({
    action,
    text: `${action.title} ${action.group} ${action.appName ?? ""} ${action.category ?? ""} ${action.shortcut ?? ""}`.toLowerCase(),
  }));
}

export function searchActions(
  index: ReturnType<typeof indexActions>,
  query: string,
  group: string,
  profile = "",
) {
  const tokens = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const ready: Action[] = [];
  const unavailable: Action[] = [];
  for (const { action, text } of index) {
    if (group !== "All actions" && action.group !== group) continue;
    if (profile && action.profile !== profile) continue;
    if (
      !tokens.every((token) =>
        token.includes("+")
          ? action.shortcut?.toLowerCase().split(/\s+/).includes(token)
          : text.includes(token),
      )
    )
      continue;
    (action.eligible ? ready : unavailable).push(action);
  }
  return [...ready, ...unavailable];
}
