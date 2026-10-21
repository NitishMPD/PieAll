// Render the actual React components to a static fixture. No app, server or bundle.
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const ts = require("typescript");
const React = require("react");
const { renderToStaticMarkup } = require("react-dom/server");
const root = path.resolve(__dirname, "..");
const cache = new Map();
function load(filename) {
  if (cache.has(filename)) return cache.get(filename).exports;
  const module = { exports: {} };
  cache.set(filename, module);
  const code = ts.transpileModule(fs.readFileSync(filename, "utf8"), {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.CommonJS,
      jsx: ts.JsxEmit.ReactJSX,
      esModuleInterop: true,
    },
  }).outputText;
  const localRequire = (name) => {
    if (name.endsWith(".css")) return {};
    if (name === "./native") return { native: {} };
    if (!name.startsWith(".")) return require(name);
    const resolved = path.resolve(path.dirname(filename), name);
    return load(
      [".tsx", ".ts"]
        .map((ext) => resolved + ext)
        .find((file) => fs.existsSync(file)),
    );
  };
  vm.runInNewContext(
    code,
    {
      module,
      exports: module.exports,
      require: localRequire,
      console,
      URLSearchParams,
      setTimeout,
    },
    { filename },
  );
  return module.exports;
}
const { App } = load(path.join(root, "web/main.tsx"));
const catalog = fs.readFileSync(path.join(root, "src/actions.rs"), "utf8");
// Fixture fields are explicit; the real registry stays native-owned at runtime.
const titles = [
  ["convert", "Convert image format"],
  ["crop", "Crop image"],
  ["resize", "Resize image"],
  ["frame", "Add framed background"],
  ["compress", "Compress image to JPEG"],
  ["rotate_left", "Rotate left"],
  ["grayscale", "Grayscale image"],
  ["strip_metadata", "Strip image metadata"],
  ["palette", "Extract color palette"],
];
for (const [id] of titles)
  if (!catalog.includes(`"${id}"`))
    throw new Error(`Stale fixture action ${id}`);
const session = {
  revision: 1,
  target: "sample.jpg",
  names: ["sample.jpg"],
  kind: "images",
  busy: null,
  editor: null,
  status: "Selection ready. Choose an action.",
  result: null,
  actions: titles.map(([id, title]) => ({
    id,
    title,
    group: "Images",
    eligible: true,
    fields: [],
  })),
  update: {
    configured: false,
    version: null,
    notes: "",
    ready: false,
    message: "Development copy. Updates are disabled.",
    progress: 0,
  },
};
if (process.argv.includes("--shortcuts")) {
  const source = JSON.parse(
    fs.readFileSync(path.join(root, "src/shortcuts.json"), "utf8"),
  );
  const library = {
    profiles: source.profiles,
    actions: source.profiles.flatMap((profile) =>
      profile.sets.flatMap((set) =>
        source.sets[set].map((binding) => ({
          ...binding,
          id: `app:${profile.id}:${binding.id}`,
          profile: profile.id,
          priority:
            profile.quick.indexOf(binding.id) < 0
              ? null
              : profile.quick.indexOf(binding.id),
        })),
      ),
    ),
  };
  const { libraryActions, indexActions, searchActions } = load(
    path.join(root, "web/action-search.ts"),
  );
  // This static fixture includes the full browse library; native sessions send only their app subset.
  session.target = "Figma";
  session.kind = "window";
  session.app = "figma";
  session.names = [];
  session.actions = [
    ...["center_window", "always_top", "fit_window", "window_details"].map(
      (id) => ({
        id,
        title: id.replaceAll("_", " "),
        group: "Window",
        fields: [],
        eligible: true,
      }),
    ),
    ...libraryActions(library, "figma"),
  ];
  const { performance } = require("node:perf_hooks");
  const startIndex = performance.now();
  const index = indexActions(session.actions);
  const indexMs = performance.now() - startIndex;
  const queries = [
    "",
    "figma layout",
    "photoshop Ctrl+M",
    "excel filter",
    "code format",
    "not-found",
  ];
  const startSearch = performance.now();
  for (let i = 0; i < 1000; i++)
    searchActions(index, queries[i % queries.length], "All actions");
  console.log(
    JSON.stringify({
      actions: index.length,
      profiles: source.profiles.length,
      indexMs,
      meanSearchMs: (performance.now() - startSearch) / 1000,
    }),
  );
}
const css =
  fs.readFileSync(path.join(root, "web/tokens.css"), "utf8") +
  fs
    .readFileSync(path.join(root, "web/styles.css"), "utf8")
    .replace(/@import[^;]+;/g, "");
const markup = renderToStaticMarkup(
  React.createElement(App, { initialSession: session }),
);
if (process.argv.includes("--shortcuts")) {
  const rows = (markup.match(/class="action-row"/g) ?? []).length;
  if (rows !== 60) throw new Error(`Expected 60 rendered rows, got ${rows}`);
  if (!markup.includes("Frame tool") || !markup.includes("All shortcuts"))
    throw new Error("Missing app ring choices");
  if (!markup.includes("Next</button>"))
    throw new Error("Missing action pagination");
  console.log(
    "Static render: 60 rows, app quick actions and accessible pagination verified.",
  );
}
const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><style>${css}html,body,#root{height:740px;min-height:0}body{min-width:640px}</style></head><body><div id="root">${markup}</div></body></html>`;
const output = path.join(root, "verification/tauri-layout.html");
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, html);
console.log(output);
