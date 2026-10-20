// External writes occur only when this script is explicitly run by the user/CI.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const {
  REPOSITORY,
  requiredAssets,
  validateDirectory,
  compareDirectories,
  newerVersion,
} = require("./release-artifacts.cjs");
const { readVersions } = require("./release-version.cjs");
const root = path.resolve(__dirname, "..");
function gh(args) {
  return execFileSync("gh", [...args, "--repo", REPOSITORY], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}
function api(endpoint) {
  return execFileSync("gh", ["api", endpoint], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}
function maybeRelease(endpoint) {
  try {
    return JSON.parse(api(`repos/${REPOSITORY}/releases/${endpoint}`));
  } catch (error) {
    if (String(error.stderr).includes("HTTP 404")) return null;
    throw error;
  }
}
function draft(version) {
  assert(
    !process.env.GITHUB_REPOSITORY ||
      process.env.GITHUB_REPOSITORY === REPOSITORY,
    "Update both app feed and publishing contracts before changing repository",
  );
  const tag = `v${version}`;
  const commit =
    process.env.GITHUB_SHA ||
    execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
    }).trim();
  let release = maybeRelease(`tags/${tag}`);
  if (!release) {
    gh([
      "release",
      "create",
      tag,
      "--target",
      commit,
      "--draft",
      "--generate-notes",
      "--title",
      `PieAll ${tag}`,
    ]);
    release = maybeRelease(`tags/${tag}`);
  }
  assert(
    release?.draft && release.target_commitish === commit,
    "Release must be an owned draft at this exact commit",
  );
  fs.mkdirSync(path.join(root, ".release"), { recursive: true });
  fs.writeFileSync(
    path.join(root, ".release/release-notes.txt"),
    release.body ?? "",
  );
  console.log(`Draft ready: ${tag}`);
}
function verifyAndPublish(version) {
  const tag = `v${version}`;
  const directory = path.join(root, `.release/v${version}/assets`);
  const downloaded = path.join(root, `.release/v${version}/downloaded`);
  validateDirectory(directory, version);
  const release = maybeRelease(`tags/${tag}`);
  const commit =
    process.env.GITHUB_SHA ||
    execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
    }).trim();
  assert(
    release?.draft && release.target_commitish === commit,
    "Refusing to update a published/unrelated release",
  );
  gh([
    "release",
    "upload",
    tag,
    ...requiredAssets(version).map((name) => path.join(directory, name)),
    "--clobber",
  ]);
  assert(
    !fs.existsSync(downloaded),
    "Readback directory already exists; use a fresh checkout for retries",
  );
  gh(["release", "download", tag, "--dir", downloaded]);
  compareDirectories(directory, downloaded, version);
  execFileSync(
    "cargo",
    [
      "run",
      "-p",
      "pieall-desktop",
      "--locked",
      "--features",
      "release-tools",
      "--bin",
      "release-verify",
      "--",
      downloaded,
    ],
    { cwd: root, stdio: "inherit" },
  );
  const latest = maybeRelease("latest");
  if (latest)
    assert(
      newerVersion(version, latest.tag_name.replace(/^v/, "")),
      "A newer/equal release is already latest; this draft will not replace it",
    );
  gh(["release", "edit", tag, "--draft=false", "--latest"]);
  console.log(`Published verified release ${tag}`);
}
if (require.main === module) {
  const version = readVersions(root).base;
  if (process.argv[2] === "draft") draft(version);
  else if (process.argv[2] === "publish") verifyAndPublish(version);
  else
    throw Error(
      "Expected draft|publish. These commands modify GitHub releases.",
    );
}
