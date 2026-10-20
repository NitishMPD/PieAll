import type { Action } from "./native";

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
        return [{ ...item, children: children.slice(0, MAX) }];
      }
      return item.action && ok.has(item.action) ? [item] : [];
    });
  const out = walk(items);
  if (out.length <= MAX) return out;
  return [
    ...out.slice(0, MAX - 1),
    sub("overflow", "More", "more", out.slice(MAX - 1, MAX * 2 - 2)),
  ];
}
