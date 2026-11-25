import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { syncVersion } from "../version.mjs";

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "office363-version-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "src-tauri"));
  const files = {
    "package.json": JSON.stringify({ name: "office363", version: "1.2.3" }, null, 2) + "\n",
    "package-lock.json": JSON.stringify({ version: "1.2.3", packages: { "": { version: "1.2.3" } } }, null, 2) + "\n",
    "src-tauri/tauri.conf.json": '{"version":"../package.json"}',
    "src-tauri/Cargo.toml": '[package]\nname = "office363"\nversion = "1.2.3"\n',
    "src-tauri/Cargo.lock": '[[package]]\nname = "dependency"\nversion = "9.0.0"\n\n[[package]]\nname = "office363"\nversion = "1.2.3"\n',
  };
  for (const [path, contents] of Object.entries(files)) writeFileSync(join(root, path), contents);
  return root;
}

test("one version change updates Cargo and both lockfiles without changing dependencies", (t) => {
  const root = fixture(t);
  assert.equal(syncVersion(root, { version: "2.10.4" }), "2.10.4");
  assert.equal(syncVersion(root, { check: true, tag: "v2.10.4" }), "2.10.4");
  assert.match(readFileSync(join(root, "src-tauri/Cargo.lock"), "utf8"), /name = "dependency"\nversion = "9.0.0"/);
  assert.equal(JSON.parse(readFileSync(join(root, "package.json"))).version, "2.10.4");
});

test("CI rejects drift without silently repairing it", (t) => {
  const root = fixture(t);
  const path = join(root, "src-tauri/Cargo.toml");
  writeFileSync(path, readFileSync(path, "utf8").replace("1.2.3", "8.0.0"));
  assert.throws(() => syncVersion(root, { check: true }), /Version mismatch/);
  assert.match(readFileSync(path, "utf8"), /8.0.0/);
  syncVersion(root);
  syncVersion(root, { check: true });
});

test("invalid versions and mismatched release tags fail before mutation", (t) => {
  const root = fixture(t);
  for (const version of ["v1.0.0", "01.0.0", "1.0", "1.0.0-beta.1", "1.0.0\n", "1.0.0;exit"]) {
    assert.throws(() => syncVersion(root, { version }), /MAJOR.MINOR.PATCH/);
  }
  assert.throws(() => syncVersion(root, { check: true, tag: "v7.0.0" }), /does not match/);
  assert.equal(syncVersion(root, { check: true }), "1.2.3");
});
