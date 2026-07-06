#!/usr/bin/env node
/**
 * Run examples/builtins/*.ny and write stdout to builtin-outputs.json for methods.html.
 * Usage: node webDocs/scripts/capture-builtin-outputs.mjs
 */
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { formatOutput } from "./lib/builtin-meta.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "../..");
const BUILTINS = path.join(ROOT, "examples", "builtins");
const OUT = path.join(__dirname, "builtin-outputs.json");
const NYRA = path.join(ROOT, "target", "release", "nyra");

function walkNy(dir, out = []) {
  for (const name of fs.readdirSync(dir)) {
    const full = path.join(dir, name);
    if (fs.statSync(full).isDirectory()) {
      walkNy(full, out);
    } else if (name.endsWith(".ny") && !name.endsWith(".typed.ny")) {
      out.push(path.relative(ROOT, full).replace(/\\/g, "/"));
    }
  }
  return out;
}

function runExample(rel) {
  const inp = rel.includes("io/input") ? "Ada\n" : undefined;
  const r = spawnSync(NYRA, ["run", rel], {
    cwd: ROOT,
    encoding: "utf8",
    input: inp,
    timeout: 60_000,
  });
  const lines = [];
  for (const line of `${r.stdout ?? ""}${r.stderr ?? ""}`.split("\n")) {
    if (
      /^\s*(Compiling|Finished)\s/.test(line) ||
      /^\s*nyra\s+/.test(line) ||
      line.startsWith("incremental:") ||
      line.includes("warning[") ||
      line.startsWith("in `") ||
      line.startsWith("   ") ||
      line.startsWith("=")
    ) {
      continue;
    }
    if (line.trim()) {
      lines.push(line);
    }
  }
  if (r.status !== 0) {
    return `(run failed — exit ${r.status})`;
  }
  return formatOutput(rel, lines.join("\n"));
}

function main() {
  if (!fs.existsSync(NYRA)) {
    console.error(`nyra not found at ${NYRA} — build the compiler first`);
    process.exit(1);
  }
  const map = {};
  for (const rel of walkNy(BUILTINS).sort()) {
    map[rel] = runExample(rel);
    console.log(rel);
  }
  fs.writeFileSync(OUT, `${JSON.stringify(map, null, 2)}\n`);
  console.log(`wrote ${OUT} (${Object.keys(map).length} entries)`);
}

main();
