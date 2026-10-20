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
const css =
  fs.readFileSync(path.join(root, "web/tokens.css"), "utf8") +
  fs
    .readFileSync(path.join(root, "web/styles.css"), "utf8")
    .replace(/@import[^;]+;/g, "");
const markup = renderToStaticMarkup(
  React.createElement(App, { initialSession: session }),
);
const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><style>${css}html,body,#root{height:740px;min-height:0}body{min-width:640px}</style></head><body><div id="root">${markup}</div></body></html>`;
const output = path.join(root, "verification/tauri-layout.html");
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, html);
console.log(output);
