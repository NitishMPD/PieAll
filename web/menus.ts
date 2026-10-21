import type { Action, Session } from "./native";

/** One ring slice: a preset action or a submenu of them. */
export interface MenuItem {
  key: string;
  label: string;
  icon: string;
  hint?: string;
  action?: string;
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

/** Only native eligible app actions enter the ring, never the browse library. */
export function contextMenu(session: Session): MenuItem[] {
  const base = menus[session.kind] ?? [];
  const shortcuts = session.actions.filter((a) => a.eligible && a.shortcut);
  if (!shortcuts.length) return base;
  const categories = new Map<string, MenuItem[]>();
  for (const action of shortcuts) {
    const category = action.category ?? "Shortcuts";
    const items = categories.get(category) ?? [];
    items.push(
      run(
        action.id,
        action.title,
        shortcutIcon(action.id),
        {},
        action.shortcut ?? undefined,
      ),
    );
    categories.set(category, items);
  }
  const app = Array.from(categories, ([name, children]) =>
    sub(`app:${name}`, name, "code", children),
  );
  const quick = shortcuts
    .filter((action) => action.priority != null)
    .sort((a, b) => a.priority! - b.priority!)
    .slice(0, 6)
    .map((action) =>
      run(
        action.id,
        action.title,
        shortcutIcon(action.id),
        {},
        action.shortcut ?? undefined,
      ),
    );
  const appMenu = quick.length
    ? [...quick, sub("app-all", "All shortcuts", "apps", app)]
    : app;
  return session.kind === "window"
    ? [...appMenu, sub("window-tools", "Window", "window", base)]
    : [...base, sub("app-shortcuts", "App shortcuts", "apps", appMenu)];
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
      return item.action && ok.has(item.action) ? [item] : [];
    });
  return paginate(walk(items));
}
