const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { stableVersion } = require("./release-version.cjs");
const REPOSITORY = "NitishMPD/PieAll";

function names(version) {
  stableVersion(version);
  return {
    installer: `PieAll_${version}_x64-setup.exe`,
    portable: `PieAll_${version}_x64-portable.zip`,
  };
}
function requiredAssets(version) {
  const { installer, portable } = names(version);
  return [
    installer,
    `${installer}.sig`,
    portable,
    `${portable}.sig`,
    "latest.json",
    "SHA256SUMS.txt",
  ].sort();
}
function manifest(version, notes, signature, date = new Date().toISOString()) {
  stableVersion(version);
  assert(
    Buffer.byteLength(notes) <= 128 * 1024,
    "Release notes exceed runtime limit",
  );
  assert(
    signature.trim().length > 0 && signature.length <= 16384,
    "Missing/oversized installer signature",
  );
  const { installer } = names(version);
  return {
    version,
    notes,
    pub_date: date,
    platforms: {
      "windows-x86_64": {
        signature: signature.trim(),
        url: `https://github.com/${REPOSITORY}/releases/download/v${version}/${installer}`,
      },
    },
  };
}
function validateManifest(value, version) {
  const expected = manifest(
    version,
    value.notes ?? "",
    value.platforms?.["windows-x86_64"]?.signature ?? "",
    value.pub_date,
  );
  assert.deepEqual(
    Object.keys(value.platforms ?? {}),
    ["windows-x86_64"],
    "Unexpected updater targets",
  );
  assert.equal(value.version, version, "Manifest version differs");
  assert.equal(
    value.platforms["windows-x86_64"].url,
    expected.platforms["windows-x86_64"].url,
    "Manifest points to an unowned installer",
  );
  assert(
    Number.isFinite(Date.parse(value.pub_date)),
    "Invalid publication timestamp",
  );
}
function sha256(file) {
  return crypto
    .createHash("sha256")
    .update(fs.readFileSync(file))
    .digest("hex");
}
function finalize(directory, version, notes) {
  const { installer } = names(version);
  const signature = fs.readFileSync(
    path.join(directory, `${installer}.sig`),
    "utf8",
  );
  const json = manifest(version, notes, signature);
  fs.writeFileSync(
    path.join(directory, "latest.json"),
    JSON.stringify(json, null, 2) + "\n",
  );
  const assets = requiredAssets(version).filter(
    (name) => name !== "SHA256SUMS.txt",
  );
  for (const name of assets)
    assert(fs.statSync(path.join(directory, name)).size > 0, `Empty ${name}`);
  fs.writeFileSync(
    path.join(directory, "SHA256SUMS.txt"),
    assets
      .map((name) => `${sha256(path.join(directory, name))}  ${name}\n`)
      .join(""),
  );
  validateDirectory(directory, version);
}
function validateDirectory(directory, version) {
  assert.deepEqual(
    fs.readdirSync(directory).sort(),
    requiredAssets(version),
    "Missing/unexpected release assets",
  );
  const value = JSON.parse(
    fs.readFileSync(path.join(directory, "latest.json"), "utf8"),
  );
  validateManifest(value, version);
  const signature = fs
    .readFileSync(
      path.join(directory, `${names(version).installer}.sig`),
      "utf8",
    )
    .trim();
  assert.equal(
    value.platforms["windows-x86_64"].signature,
    signature,
    "Manifest signature differs from asset",
  );
  const sums = fs
    .readFileSync(path.join(directory, "SHA256SUMS.txt"), "utf8")
    .trimEnd()
    .split("\n");
  const expected = requiredAssets(version).filter(
    (name) => name !== "SHA256SUMS.txt",
  );
  assert.equal(sums.length, expected.length, "Checksum inventory differs");
  for (const [index, name] of expected.entries())
    assert.equal(
      sums[index],
      `${sha256(path.join(directory, name))}  ${name}`,
      `Checksum mismatch: ${name}`,
    );
  return value;
}
function compareDirectories(local, downloaded, version) {
  validateDirectory(local, version);
  validateDirectory(downloaded, version);
  for (const name of requiredAssets(version))
    assert.equal(
      sha256(path.join(local, name)),
      sha256(path.join(downloaded, name)),
      `Uploaded asset differs: ${name}`,
    );
}
function newerVersion(candidate, published) {
  const a = stableVersion(candidate),
    b = stableVersion(published);
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] > b[i];
  return false;
}
if (require.main === module) {
  const [command, directory, version, extra] = process.argv.slice(2);
  if (command === "finalize")
    finalize(directory, version, fs.readFileSync(extra, "utf8"));
  else if (command === "verify") validateDirectory(directory, version);
  else if (command === "compare") compareDirectories(directory, extra, version);
  else
    throw Error(
      "Expected finalize|verify|compare DIRECTORY VERSION [NOTES_FILE|DOWNLOADED_DIRECTORY]",
    );
  console.log(`Release assets ${command}: ${version}`);
}
module.exports = {
  REPOSITORY,
  names,
  requiredAssets,
  manifest,
  validateManifest,
  finalize,
  validateDirectory,
  compareDirectories,
  newerVersion,
};
