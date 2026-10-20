const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const {
  names,
  manifest,
  validateManifest,
  finalize,
  validateDirectory,
  compareDirectories,
  newerVersion,
} = require("./release-artifacts.cjs");

test("manifest is compatible with the exact stable runtime URL/target contract", () => {
  const data = manifest(
    "0.1.42",
    "Notes\nwith lines",
    "signature",
    "2026-10-20T00:00:00Z",
  );
  validateManifest(data, "0.1.42");
  data.platforms["windows-x86_64"].url = "https://example.com/setup.exe";
  assert.throws(() => validateManifest(data, "0.1.42"));
  assert.throws(() =>
    manifest("0.1.42", "x".repeat(128 * 1024 + 1), "signature"),
  );
  assert.throws(() => manifest("0.1.42-beta", "", "signature"));
});
test("missing, altered and extra uploaded artifacts cannot pass readback verification", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pieall-assets-"));
  t.after(() => fs.rmSync(root, { recursive: true }));
  const local = path.join(root, "local"),
    downloaded = path.join(root, "download");
  fs.mkdirSync(local);
  const { installer, portable } = names("0.1.8");
  for (const name of [
    installer,
    `${installer}.sig`,
    portable,
    `${portable}.sig`,
  ])
    fs.writeFileSync(path.join(local, name), `fixture ${name}`);
  finalize(local, "0.1.8", "Release notes");
  fs.cpSync(local, downloaded, { recursive: true });
  compareDirectories(local, downloaded, "0.1.8");
  fs.appendFileSync(path.join(downloaded, portable), "tampered");
  assert.throws(() => compareDirectories(local, downloaded, "0.1.8"));
  fs.writeFileSync(
    path.join(downloaded, portable),
    fs.readFileSync(path.join(local, portable)),
  );
  fs.writeFileSync(path.join(downloaded, "secret.key"), "unwanted");
  assert.throws(() => validateDirectory(downloaded, "0.1.8"));
  fs.unlinkSync(path.join(downloaded, "secret.key"));
  fs.unlinkSync(path.join(downloaded, `${installer}.sig`));
  assert.throws(() => validateDirectory(downloaded, "0.1.8"));
});
test("older or equal runs cannot replace a newer latest release", () => {
  assert(newerVersion("0.1.10", "0.1.9"));
  assert(newerVersion("1.0.0", "0.99.9"));
  assert(!newerVersion("0.1.8", "0.1.9"));
  assert(!newerVersion("0.1.9", "0.1.9"));
});
