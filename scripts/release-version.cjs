const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

function stableVersion(value) {
  assert(
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(value),
    "Version must be canonical stable major.minor.patch",
  );
  const numbers = value.split(".").map(Number);
  assert(numbers.every(Number.isSafeInteger), "Version exceeds integer bounds");
  assert(
    numbers[0] <= 255 && numbers[1] <= 255 && numbers[2] <= 65535,
    "Version exceeds Windows installer limits",
  );
  return numbers;
}

function releaseVersion(base, run) {
  const [major, minor, patch] = stableVersion(base);
  assert(
    /^[1-9]\d*$/.test(String(run)),
    "GITHUB_RUN_NUMBER must be a positive integer",
  );
  const next = `${major}.${minor}.${patch + Number(run)}`;
  stableVersion(next);
  return next;
}

function packageBlock(text, header, name) {
  const expression =
    header === "lock"
      ? /\[\[package\]\][\s\S]*?(?=\n\[\[package\]\]|$)/g
      : /\[package\][\s\S]*?(?=\n\[|$)/g;
  const blocks = [...text.matchAll(expression)].filter(
    (match) =>
      new RegExp(`^name\\s*=\\s*"${name}"\\s*$`, "m").test(match[0]) &&
      !/^source\s*=/m.test(match[0]),
  );
  assert.equal(blocks.length, 1, `Expected one local ${name} ${header} block`);
  const match = /^version\s*=\s*"([^"]+)"\s*$/m.exec(blocks[0][0]);
  assert(match, `Missing version for ${name}`);
  return { block: blocks[0][0], version: match[1] };
}

function readVersions(root) {
  const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
  const files = new Map(
    [
      "package.json",
      "src-tauri/tauri.conf.json",
      "Cargo.toml",
      "src-tauri/Cargo.toml",
      "Cargo.lock",
    ].map((file) => [file, read(file)]),
  );
  const base = JSON.parse(files.get("package.json")).version;
  stableVersion(base);
  assert.equal(
    JSON.parse(files.get("src-tauri/tauri.conf.json")).version,
    base,
    "Tauri version differs",
  );
  for (const [file, name] of [
    ["Cargo.toml", "pieall"],
    ["src-tauri/Cargo.toml", "pieall-desktop"],
  ])
    assert.equal(
      packageBlock(files.get(file), "manifest", name).version,
      base,
      `${file} differs`,
    );
  for (const name of ["pieall", "pieall-desktop"])
    assert.equal(
      packageBlock(files.get("Cargo.lock"), "lock", name).version,
      base,
      `${name} lock version differs`,
    );
  return { files, base };
}

function versionPlan(root, run) {
  const { files, base } = readVersions(root);
  const version = releaseVersion(base, run);
  for (const file of ["package.json", "src-tauri/tauri.conf.json"]) {
    const text = files.get(file);
    assert.equal(
      [...text.matchAll(/^  "version": "[^"]+"/gm)].length,
      1,
      `Expected one root version in ${file}`,
    );
    files.set(file, text.replace(/^(  "version": ")[^"]+"/m, `$1${version}"`));
  }
  for (const [file, name] of [
    ["Cargo.toml", "pieall"],
    ["src-tauri/Cargo.toml", "pieall-desktop"],
    ["Cargo.lock", "pieall"],
    ["Cargo.lock", "pieall-desktop"],
  ]) {
    const text = files.get(file);
    const { block } = packageBlock(
      text,
      file === "Cargo.lock" ? "lock" : "manifest",
      name,
    );
    files.set(
      file,
      text.replace(
        block,
        block.replace(/^(version\s*=\s*")[^"]+"/m, `$1${version}"`),
      ),
    );
  }
  // pnpm does not store the root project version. Its importer/resolutions stay unchanged.
  return { files, base, version, run: String(run) };
}

function applyVersion(root, run) {
  const receipt = path.join(root, ".release/version.json");
  if (fs.existsSync(receipt)) {
    const previous = JSON.parse(fs.readFileSync(receipt, "utf8"));
    assert.equal(
      previous.run,
      String(run),
      "Use a fresh checkout for a different release run",
    );
    assert.equal(
      readVersions(root).base,
      previous.version,
      "Release receipt does not match current files",
    );
    return previous.version;
  }
  const plan = versionPlan(root, run); // Validate every input before changing any file.
  for (const [file, text] of plan.files)
    fs.writeFileSync(path.join(root, file), text);
  fs.mkdirSync(path.dirname(receipt), { recursive: true });
  fs.writeFileSync(
    receipt,
    JSON.stringify(
      { base: plan.base, version: plan.version, run: plan.run },
      null,
      2,
    ) + "\n",
  );
  return plan.version;
}

if (require.main === module) {
  const root = path.resolve(__dirname, "..");
  if (process.argv.includes("--check"))
    console.log(`Versions synchronized: ${readVersions(root).base}`);
  else
    console.log(
      `Release version: ${applyVersion(root, process.env.GITHUB_RUN_NUMBER)}`,
    );
}
module.exports = {
  stableVersion,
  releaseVersion,
  packageBlock,
  readVersions,
  versionPlan,
  applyVersion,
};
