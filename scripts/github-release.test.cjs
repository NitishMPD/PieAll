const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

const source = fs.readFileSync(require.resolve("./github-release.cjs"), "utf8");
const commit = "f34741e1fc39c02cf1346eb7aee22d3115a721bd";
const owned = {
  tag_name: "v0.1.12",
  target_commitish: commit,
  draft: true,
  body: "Generated release notes",
};

function runDraft(releases, created = owned) {
  const calls = [];
  const writes = [];
  vm.runInNewContext(source + '\ndraft("0.1.12");', {
    __dirname,
    module: {},
    process: {
      env: { GITHUB_SHA: commit, GITHUB_REPOSITORY: "NitishMPD/PieAll" },
    },
    console: { log() {} },
    require(name) {
      if (name === "node:fs") {
        return {
          mkdirSync() {},
          writeFileSync(file, text) {
            writes.push([file, text]);
          },
        };
      }
      if (name === "node:child_process") {
        return {
          execFileSync(command, args, options) {
            assert.equal(command, "gh");
            assert.equal(args[0], "api");
            calls.push(args);
            if (args[1].endsWith("?per_page=100"))
              return JSON.stringify(releases);
            assert.equal(args[1], "repos/NitishMPD/PieAll/releases");
            assert.deepEqual(Array.from(args.slice(2)), [
              "--method",
              "POST",
              "--input",
              "-",
            ]);
            assert.deepEqual(JSON.parse(options.input), {
              tag_name: owned.tag_name,
              target_commitish: commit,
              draft: true,
              generate_release_notes: true,
              name: "PieAll v0.1.12",
            });
            return JSON.stringify(created);
          },
        };
      }
      return require(name);
    },
  });
  return { calls, writes };
}

test("uses the created draft response even when the release list is stale", () => {
  const { calls, writes } = runDraft([]);
  assert.equal(calls.length, 2);
  assert.equal(writes.length, 1);
  assert.equal(writes[0][1], owned.body);
});

test("resumes an existing owned draft without creating another release", () => {
  const { calls, writes } = runDraft([owned]);
  assert.equal(calls.length, 1);
  assert.equal(writes[0][1], owned.body);
});

test("rejects published releases and mismatched tags or commits", () => {
  for (const patch of [
    { draft: false },
    { target_commitish: "another-commit" },
    { tag_name: "v0.1.13" },
  ]) {
    assert.throws(
      () => runDraft([], { ...owned, ...patch }),
      /owned draft at this exact commit/,
    );
  }
  assert.throws(
    () => runDraft([{ ...owned, draft: false }]),
    /owned draft at this exact commit/,
  );
  assert.throws(
    () => runDraft([{ ...owned, target_commitish: "another-commit" }]),
    /owned draft at this exact commit/,
  );
});
