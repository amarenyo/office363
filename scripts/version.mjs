import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

// package.json is authoritative. Cargo and lockfiles contain generated copies.
export function syncVersion(root, { check = false, version, tag } = {}) {
  const read = (path) => readFileSync(resolve(root, path), "utf8");
  const manifest = JSON.parse(read("package.json"));
  version ??= manifest.version;
  if (typeof version !== "string" || version !== version.trim() || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error("Use a stable MAJOR.MINOR.PATCH version without a v prefix.");
  }
  if (tag !== undefined && tag !== `v${version}`) {
    throw new Error(`Tag ${tag} does not match package.json (v${version}).`);
  }
  const config = JSON.parse(read("src-tauri/tauri.conf.json"));
  if (config.version !== "../package.json") {
    throw new Error("Tauri must read its version from ../package.json.");
  }
  manifest.version = version;
  const lock = JSON.parse(read("package-lock.json"));
  lock.version = version;
  lock.packages[""].version = version;
  const replacements = new Map([
    ["package.json", JSON.stringify(manifest, null, 2) + "\n"],
    ["package-lock.json", JSON.stringify(lock, null, 2) + "\n"],
  ]);
  for (const [path, pattern] of [
    ["src-tauri/Cargo.toml", /(\[package\][\s\S]*?\nversion = ")[^"]+("\n)/],
    ["src-tauri/Cargo.lock", /(\[\[package\]\]\nname = "office363"\nversion = ")[^"]+("\n)/],
  ]) {
    const contents = read(path);
    if (!pattern.test(contents)) throw new Error(`Cannot find application version in ${path}.`);
    replacements.set(path, contents.replace(pattern, (_, before, after) => before + version + after));
  }
  const changed = [...replacements].filter(([path, contents]) => read(path) !== contents);
  if (check && changed.length) {
    throw new Error(`Version mismatch: ${changed.map(([path]) => path).join(", ")}. Run npm run version:sync.`);
  }
  for (const [path, contents] of changed) writeFileSync(resolve(root, path), contents);
  return version;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2);
    if (args.length > 1 || (args[0]?.startsWith("-") && args[0] !== "--check")) {
      throw new Error("Usage: node scripts/version.mjs [--check | MAJOR.MINOR.PATCH]");
    }
    const version = syncVersion(fileURLToPath(new URL("../", import.meta.url)), {
      check: args[0] === "--check",
      version: args[0] === "--check" ? undefined : args[0],
      tag: process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : undefined,
    });
    console.log(version);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
