#!/usr/bin/env node
/**
 * Embed tabbed code examples (easy / typed) into stdlib.html and methods.html.
 * methods.html: method name titles + captured stdout (see capture-builtin-outputs.mjs).
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { tabBlock } from "./lib/code-tabs.mjs";
import { methodsGalleryBlock, stdlibGalleryBlock } from "./lib/method-gallery.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEBDOCS = path.resolve(__dirname, "..");
const ROOT = path.resolve(WEBDOCS, "..");
const BUILTINS = path.join(ROOT, "examples", "builtins");
const OUTPUTS_JSON = path.join(__dirname, "builtin-outputs.json");
const START = "<!-- BUILTIN_CODE_TABS_START -->";
const END = "<!-- BUILTIN_CODE_TABS_END -->";

function rel(p) {
  return path.relative(ROOT, p).replace(/\\/g, "/");
}

function walkNy(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const name of fs.readdirSync(dir)) {
    const full = path.join(dir, name);
    const st = fs.statSync(full);
    if (st.isDirectory()) walkNy(full, out);
    else if (name.endsWith(".ny") && !name.endsWith(".typed.ny")) out.push(full);
  }
  return out;
}

function collectPairs() {
  const pairs = [];
  for (const plain of walkNy(BUILTINS)) {
    const typed = path.join(
      path.dirname(plain),
      `${path.basename(plain, ".ny")}.typed.ny`,
    );
    if (!fs.existsSync(typed)) continue;
    const title = rel(plain);
    const id = title.replace(/[^a-zA-Z0-9]+/g, "-").replace(/^-|-$/g, "");
    pairs.push({
      id,
      title,
      plain: fs.readFileSync(plain, "utf8").trimEnd(),
      typed: fs.readFileSync(typed, "utf8").trimEnd(),
    });
  }
  return pairs.sort((a, b) => a.title.localeCompare(b.title));
}

function loadOutputs() {
  if (!fs.existsSync(OUTPUTS_JSON)) {
    console.warn(`warning: ${OUTPUTS_JSON} missing — run node webDocs/scripts/capture-builtin-outputs.mjs`);
    return {};
  }
  return JSON.parse(fs.readFileSync(OUTPUTS_JSON, "utf8"));
}

function galleryHtml(pairs, intro, blockFn) {
  return `
<section id="builtin-examples-gallery">
  <h3 id="builtins-gallery">Runnable examples — easy vs typed</h3>
  ${intro}
  ${pairs.map((p) => blockFn(p)).join("\n")}
</section>`;
}

function embedGallery(htmlPath, gallery, label) {
  let html = fs.readFileSync(htmlPath, "utf8");
  const startIdx = html.indexOf(START);
  const endIdx = html.indexOf(END);
  if (startIdx === -1 || endIdx === -1 || endIdx < startIdx) {
    console.error(`missing BUILTIN_CODE_TABS markers in ${label}`);
    process.exit(1);
  }
  const before = html.slice(0, startIdx + START.length);
  const after = html.slice(endIdx);
  html = `${before}\n${gallery}\n${after}`;
  fs.writeFileSync(htmlPath, html);
}

function main() {
  const pairs = collectPairs();
  const outputs = loadOutputs();

  const stdlibIntro =
    '<p class="lead">Same program, two styles. Default tab is <strong>Without types</strong> — switch to <strong>With types</strong> when you want explicit annotations.</p>';
  const methodsIntro =
    '<p class="lead">Each block is named after the <strong>method or builtin</strong> it demonstrates. Run: <code>nyra run examples/builtins/…</code>. <strong>Output</strong> shows what prints to stdout.</p>';

  embedGallery(
    path.join(WEBDOCS, "stdlib.html"),
    galleryHtml(pairs, stdlibIntro, (p) => stdlibGalleryBlock(p)),
    "stdlib.html",
  );
  embedGallery(
    path.join(WEBDOCS, "methods.html"),
    galleryHtml(pairs, methodsIntro, (p) => methodsGalleryBlock(p, outputs[p.title] ?? "")),
    "methods.html",
  );
  console.log(`builtin snippets: ${pairs.length} tabbed examples → stdlib.html + methods.html`);
}

main();
