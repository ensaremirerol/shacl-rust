import { copyFile, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const crateRoot = resolve(process.cwd(), "crates/shacl-wasm");
const pkgDir = resolve(crateRoot, "pkg");
const npmDir = resolve(crateRoot, "npm");

const packageJsonPath = resolve(pkgDir, "package.json");
const packageJsonRaw = await readFile(packageJsonPath, "utf8");
const generatedPackage = JSON.parse(packageJsonRaw);

const mergedPackage = {
  ...generatedPackage,
  name: "@ensaremirerol/shacl-wasm",
  description: "WASM/JS/TS bindings for shacl-rust",
  type: "module",
  main: "index.js",
  module: "index.js",
  types: "index.d.ts",
  files: ["*.js", "*.d.ts", "*.wasm", "LICENSE"],
  sideEffects: false,
  keywords: ["shacl", "rdf", "wasm", "validation", "typescript"],
  license: "MIT",
};

await writeFile(packageJsonPath, `${JSON.stringify(mergedPackage, null, 2)}\n`, "utf8");
await copyFile(resolve(npmDir, "index.js"), resolve(pkgDir, "index.js"));
await copyFile(resolve(npmDir, "index.d.ts"), resolve(pkgDir, "index.d.ts"));

console.log("Prepared npm package at crates/shacl-wasm/pkg");
