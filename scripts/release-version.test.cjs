const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const {
  releaseVersion,
  versionPlan,
  applyVersion,
} = require("./release-version.cjs");

function fixture(t, eol = "\n") {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pieall-version-"));
  t.after(() => fs.rmSync(root, { recursive: true }));
  fs.mkdirSync(path.join(root, "src-tauri"));
  for (const file of ["package.json", "src-tauri/tauri.conf.json"])
    fs.writeFileSync(
      path.join(root, file),
      JSON.stringify(
        { version: "1.2.3", other: "unchanged" },
        null,
        2,
      ).replaceAll("\n", eol) + eol,
    );
  for (const [file, name] of [
    ["Cargo.toml", "pieall"],
    ["src-tauri/Cargo.toml", "pieall-desktop"],
  ])
    fs.writeFileSync(
      path.join(root, file),
      `[package]\nname = "${name}"\nversion = "1.2.3"\n\n[dependencies]\nserde = "=1.0.229"\n`.replaceAll(
        "\n",
        eol,
      ),
    );
  fs.writeFileSync(
    path.join(root, "Cargo.lock"),
    `version = 4\n\n[[package]]\nname = "dependency"\nversion = "9.8.7"\nsource = "registry+example"\nchecksum = "untouched"\n\n[[package]]\nname = "pieall"\nversion = "1.2.3"\n\n[[package]]\nname = "pieall-desktop"\nversion = "1.2.3"\n`.replaceAll(
      "\n",
      eol,
    ),
  );
  fs.writeFileSync(
    path.join(root, "pnpm-lock.yaml"),
    "resolved dependencies stay identical",
  );
  return root;
}

test("base patch plus run number respects stable Windows bounds", () => {
  assert.equal(releaseVersion("1.2.3", "10"), "1.2.13");
  assert.equal(releaseVersion("255.255.65534", "1"), "255.255.65535");
  for (const pair of [
    ["0.1.0-beta", "1"],
    ["01.2.3", "1"],
    ["256.0.0", "1"],
    ["1.256.0", "1"],
    ["1.2.65535", "1"],
    ["1.2.3", "0"],
    ["1.2.3", "1.5"],
    ["1.2.3", "9007199254740992"],
  ])
    assert.throws(() => releaseVersion(...pair));
});
for (const eol of ["\n", "\r\n"])
  test(`synchronizes local versions and preserves resolutions/EOL ${JSON.stringify(eol)}`, (t) => {
    const root = fixture(t, eol);
    const plan = versionPlan(root, "7");
    assert.equal(plan.version, "1.2.10");
    assert.match(plan.files.get("Cargo.lock"), /version = "9\.8\.7"/);
    assert.match(plan.files.get("Cargo.lock"), /checksum = "untouched"/);
    for (const content of plan.files.values())
      if (eol === "\r\n") assert(!/(?<!\r)\n/.test(content));
    assert.equal(applyVersion(root, "7"), "1.2.10");
    assert.equal(applyVersion(root, "7"), "1.2.10");
    assert.throws(() => applyVersion(root, "8"));
    assert.equal(
      fs.readFileSync(path.join(root, "pnpm-lock.yaml"), "utf8"),
      "resolved dependencies stay identical",
    );
  });
test("mismatched/duplicate packages fail before any mutation", (t) => {
  const root = fixture(t);
  const before = fs.readFileSync(path.join(root, "package.json"), "utf8");
  fs.appendFileSync(
    path.join(root, "Cargo.lock"),
    '\n[[package]]\nname = "pieall"\nversion = "9.0.0"\n',
  );
  assert.throws(() => applyVersion(root, "3"));
  assert.equal(
    fs.readFileSync(path.join(root, "package.json"), "utf8"),
    before,
  );
});
