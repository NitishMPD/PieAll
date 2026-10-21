import type { Action, Session } from "./native";
import { contextualActions } from "./action-search";
import { getPrefs, type Prefs } from "./prefs";

/** One ring slice: a preset action or a submenu of them. */
export interface MenuItem {
  key: string;
  label: string;
  icon: string;
  hint?: string;
  action?: string;
  browse?: boolean;
  params?: Record<string, string>;
  children?: MenuItem[];
}

const MAX = 8;
const shortcutIcons: Record<string, string> = {
  "frame-tool": "frame",
  "rectangle-tool": "crop",
  "add-auto-layout": "grid",
  "quick-actions": "search",
  "group-selection": "grid",
  export: "open",
  brush: "images",
  "free-transform": "resize",
  "layer-via-copy": "duplicate",
  "image-size": "resize",
  "select-and-mask": "crop",
  "save-a-copy": "copy",
  "new-tab": "window",
  "reopen-closed-tab": "window",
  downloads: "open",
  "find-on-page": "search",
  "bookmark-page": "pin",
  "new-folder": "files",
  rename: "text",
  properties: "info",
  "address-bar": "link",
  "search-folder": "search",
  "details-view": "grid",
  "command-palette": "search",
  "quick-open": "open",
  "format-document": "code",
  "toggle-line-comment": "code",
  "search-files": "search",
  "focus-terminal": "terminal",
  terminal: "terminal",
  copy: "copy",
  paste: "clipboard",
  "new-note": "text",
  "new-page": "text",
  find: "search",
  search: "search",
};
function shortcutIcon(id: string): string {
  return shortcutIcons[id.split(":").at(-1) ?? ""] ?? "apps";
}

function run(
  action: string,
  label: string,
  icon: string,
  params: Record<string, string> = {},
  hint?: string,
): MenuItem {
  const variant = Object.values(params).join("-");
  return {
    key: variant ? `${action}:${variant}` : action,
    label,
    icon,
    action,
    params,
    hint,
  };
}
function sub(
  key: string,
  label: string,
  icon: string,
  children: MenuItem[],
): MenuItem {
  return { key, label, icon, children, hint: "More choices" };
}
// Every preset runs in place: outputs land beside the original with a clear name.
export const menus: Record<string, MenuItem[]> = {
  images: [
    run("compress", "Compress 50%", "compress", { quality: "50" }),
    run("resize", "Half size", "resize", { scale: "50" }, "50% width & height"),
    sub("convert", "Convert", "convert", [
      run("convert", "PNG", "images", { format: "png" }),
      run(
        "convert",
        "JPG",
        "images",
        { format: "jpg", background: "FFFFFF" },
        "White background",
      ),
      run("convert", "WebP", "images", { format: "webp" }),
    ]),
    sub("rotate", "Rotate", "rotate", [
      run("rotate_right", "Rotate right", "rotate"),
      run("flip_h", "Flip across", "flip"),
      run("rotate_left", "Rotate left", "rotate-left"),
      run("flip_v", "Flip down", "flip-v"),
    ]),
    run("copy_pixels", "Copy image", "copy"),
    run("strip_metadata", "Remove info", "clean", {}, "Strip metadata"),
    run("images_pdf", "Make PDF", "pdf"),
    sub("more", "More", "more", [
      run("compress", "Compress 80%", "compress", { quality: "80" }),
      run("resize", "Quarter size", "resize", { scale: "25" }),
      run("grayscale", "Grayscale", "grayscale"),
      run("frame", "Add backdrop", "frame"),
      run("contact_sheet", "Contact sheet", "grid"),
      run("palette", "Copy colors", "palette"),
      run("create_zip", "Zip", "archives"),
      run("copy_paths", "Copy path", "copy"),
    ]),
  ],
  files: [
    run("copy_paths", "Copy path", "copy"),
    run("create_zip", "Zip", "archives"),
    run("extract_zip", "Extract here", "extract"),
    run("duplicate", "Duplicate", "duplicate"),
    run("terminal", "Terminal here", "terminal"),
    run("sha256", "Copy SHA-256", "hash"),
    run("folder_size", "Folder size", "files"),
    run("find_duplicates", "Find duplicates", "duplicate"),
    run("list_zip", "List contents", "text"),
    run("test_zip", "Test ZIP", "check"),
  ],
  text: [
    run("upper", "UPPERCASE", "case"),
    run("lower", "lowercase", "case"),
    run("title", "Title Case", "case"),
    run("trim", "Trim spaces", "trim"),
    run("sort", "Sort lines", "sort"),
    run("dedupe", "Unique lines", "sort"),
    run("json_format", "Format JSON", "json"),
    sub("more", "More", "more", [
      run("json_minify", "Minify JSON", "json"),
      run("url_encode", "URL encode", "link"),
      run("url_decode", "URL decode", "link"),
      run("base64_encode", "Base64 encode", "code"),
      run("base64_decode", "Base64 decode", "code"),
      run("count", "Count words", "info"),
    ]),
  ],
  window: [
    run("center_window", "Center", "center"),
    run("always_top", "Pin on top", "pin"),
    run("fit_window", "Fill screen", "resize"),
    run("window_details", "Window info", "info"),
  ],
};

const KIND_GROUPS: Record<string, string[]> = {
  images: ["Images", "PDF"],
  files: ["Files", "Archives", "PDF"],
  text: ["Text"],
  window: ["Window"],
};
const FIXED_APP_SLOTS = 6;

/** Context a layout is saved under: a file type, or the captured app. */
export function contextKey(session: Pick<Session, "kind" | "app">): string {
  return session.kind === "window" && session.app
    ? `app:${session.app}`
    : session.kind;
}

/** An app command as a ring slice. */
export function shortcutItem(action: Action): MenuItem {
  return run(
    action.id,
    action.title,
    shortcutIcon(action.id),
    {},
    action.shortcut ?? undefined,
  );
}

/** App commands in the order the built-in ring ranks them. */
export function appItems(actions: Action[]): MenuItem[] {
  return actions
    .filter((action) => action.shortcut)
    .sort((a, b) => (a.priority ?? Infinity) - (b.priority ?? Infinity))
    .map(shortcutItem);
}

/** Every slice a file-type ring can hold: its presets, their choices, and plain registry actions. */
export function kindPool(kind: string, actions: Action[]): MenuItem[] {
  const pool = new Map<string, MenuItem>();
  const add = (item: MenuItem) => {
    if (!pool.has(item.key)) pool.set(item.key, item);
    item.children?.forEach(add);
  };
  (menus[kind] ?? []).forEach(add);
  for (const action of actions)
    if (
      !action.shortcut &&
      !action.id.startsWith("app:") &&
      !action.fields.length &&
      KIND_GROUPS[kind]?.includes(action.group)
    )
      add(run(action.id, action.title, action.group.toLowerCase()));
  return [...pool.values()];
}

/** The slice keys a context shows before the user edits it. */
export function defaultLayout(context: string, actions: Action[]): string[] {
  return context.startsWith("app:")
    ? appItems(actions)
        .slice(0, FIXED_APP_SLOTS)
        .map((i) => i.key)
    : (menus[context] ?? []).map((i) => i.key);
}

function custom(
  context: string,
  pool: MenuItem[],
  prefs: Prefs,
): MenuItem[] | null {
  const layout = prefs.layouts[context];
  if (!layout) return null;
  const byKey = new Map(pool.map((item) => [item.key, item]));
  return layout.flatMap((key) => byKey.get(key) ?? []);
}

/** Selection tools take precedence; app commands never become category trees. */
export function contextMenu(
  session: Session,
  prefs: Prefs = getPrefs(),
): MenuItem[] {
  const base =
    custom(session.kind, kindPool(session.kind, session.actions), prefs) ??
    menus[session.kind] ??
    [];
  if (session.kind !== "window") return base;
  const appActions = contextualActions(session.actions, session.app);
  const shortcuts =
    custom(contextKey(session), appItems(appActions), prefs) ??
    appItems(appActions).slice(0, FIXED_APP_SLOTS);
  if (!shortcuts.length) return base;
  return [
    ...shortcuts,
    {
      key: "search-actions",
      label: "Search actions",
      icon: "search",
      browse: true,
      hint: "Search this app's shortcuts",
    },
    sub("window-tools", "Window", "window", base),
  ];
}

/** Recursively page every level; no catalog entry may disappear past a slice limit. */
function paginate(items: MenuItem[], page = 0): MenuItem[] {
  if (items.length <= MAX) return items;
  return [
    ...items.slice(0, MAX - 1),
    sub(
      `overflow:${page}`,
      "More",
      "more",
      paginate(items.slice(MAX - 1), page + 1),
    ),
  ];
}

/** Keep long app command names inside their wedge; the accessible label stays complete. */
export function labelLines(label: string): string[] {
  if (label.length <= 16) return [label];
  const split = label.lastIndexOf(" ", 16);
  const at = split > 0 ? split : 16;
  const rest = label.slice(at).trim();
  return [
    label.slice(0, at),
    rest.length > 16 ? `${rest.slice(0, 15).trimEnd()}…` : rest,
  ];
}

/**
 * Keeps only what the native registry says fits this selection, minus
 * conversions to the selection's own format. Empty submenus
 * vanish, one-choice submenus collapse into their choice, and overflow beyond
 * eight slices moves into a final "More" slice.
 */
export function resolve(
  items: MenuItem[],
  actions: Action[],
  names: string[] = [],
): MenuItem[] {
  const ok = new Set(actions.filter((a) => a.eligible).map((a) => a.id));
  // Converting to the format every selected file already has would do nothing.
  const formats = new Set(
    names.map((n) => {
      const ext = n.split(".").pop()?.toLowerCase() ?? "";
      return ext === "jpeg" ? "jpg" : ext;
    }),
  );
  const pointless = (item: MenuItem) =>
    item.action === "convert" &&
    formats.size === 1 &&
    formats.has(item.params?.format ?? "");
  const walk = (list: MenuItem[]): MenuItem[] =>
    list.flatMap((item) => {
      if (pointless(item)) return [];
      if (item.children) {
        const children = walk(item.children);
        if (children.length < 2) return children;
        return [{ ...item, children: paginate(children) }];
      }
      return item.browse || (item.action && ok.has(item.action)) ? [item] : [];
    });
  return paginate(walk(items));
}
