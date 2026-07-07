#!/usr/bin/env node
/**
 * Embed tabbed code examples (easy / typed) into the webDocs galleries.
 *
 * Examples under examples/builtins/ are grouped into named sections and routed
 * to one or more pages (see lib/gallery-routes.mjs). methods.html carries every
 * section (its TOC links into them); stdlib.html shows the stdlib subset.
 * After embedding, prints a report of where each example landed so contributors
 * know which page + section + anchor to look at.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { escapeHtml } from "./lib/code-tabs.mjs";
import { methodSlug } from "./lib/builtin-meta.mjs";
import { methodsGalleryBlock, stdlibGalleryBlock } from "./lib/method-gallery.mjs";
import { PAGES, SECTION_ORDER, routeFor } from "./lib/gallery-routes.mjs";

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
    const route = routeFor(title);
    pairs.push({
      id,
      title,
      plain: fs.readFileSync(plain, "utf8").trimEnd(),
      typed: fs.readFileSync(typed, "utf8").trimEnd(),
      category: route.category,
      section: route.section,
      pages: route.pages,
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

function sectionSlug(section) {
  return `sec-${section.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")}`;
}

/** Anchor id for an example on a given page (matches the block's id="…"). */
function anchorFor(pageKey, pair) {
  return pageKey === "stdlib" ? `ex-${pair.id}` : `ex-${methodSlug(pair.title)}`;
}

function orderedSections(sectionSet) {
  const known = SECTION_ORDER.filter((s) => sectionSet.has(s));
  const extra = [...sectionSet].filter((s) => !SECTION_ORDER.includes(s));
  return [...known, ...extra];
}

function sectionsHtml(pageKey, pairs, outputs) {
  const blockFn =
    pageKey === "stdlib"
      ? (p, out) => stdlibGalleryBlock(p, out)
      : (p, out) => methodsGalleryBlock(p, out);

  const bySection = new Map();
  for (const p of pairs) {
    if (!bySection.has(p.section)) bySection.set(p.section, []);
    bySection.get(p.section).push(p);
  }

  const parts = [];
  for (const section of orderedSections(new Set(bySection.keys()))) {
    const items = bySection
      .get(section)
      .sort((a, b) => a.title.localeCompare(b.title));
    parts.push(`
<section class="builtin-section" id="${sectionSlug(section)}">
  <h3 class="builtin-section-title">${escapeHtml(section)}</h3>
  ${items.map((p) => blockFn(p, outputs[p.title] ?? "")).join("\n")}
</section>`);
  }
  return parts.join("\n");
}

function galleryHtml(intro, sectionsMarkup) {
  return `
<section id="builtin-examples-gallery">
  <h3 id="builtins-gallery">Runnable examples — easy vs typed</h3>
  ${intro}
  ${sectionsMarkup}
</section>`;
}

function embedGallery(htmlPath, gallery, label) {
  if (!fs.existsSync(htmlPath)) {
    console.error(`  ! ${label}: file not found (${htmlPath}) — skipped`);
    return false;
  }
  let html = fs.readFileSync(htmlPath, "utf8");
  const startIdx = html.indexOf(START);
  const endIdx = html.indexOf(END);
  if (startIdx === -1 || endIdx === -1 || endIdx < startIdx) {
    console.error(`  ! ${label}: missing BUILTIN_CODE_TABS markers — skipped`);
    return false;
  }
  const before = html.slice(0, startIdx + START.length);
  const after = html.slice(endIdx);
  html = `${before}\n${gallery}\n${after}`;
  fs.writeFileSync(htmlPath, html);
  return true;
}

const INTROS = {
  stdlib:
    '<p class="lead">Same program, two styles. Default tab is <strong>Without types</strong> — switch to <strong>With types</strong> when you want explicit annotations.</p>',
  methods:
    '<p class="lead">Grouped by category. Each block is named after the <strong>method or builtin</strong> it demonstrates. Run: <code>nyra run examples/builtins/…</code>. <strong>Output</strong> shows what prints to stdout.</p>',
};

function printReport(pairs, embedded) {
  console.log("\nbuiltin gallery routing:");
  for (const pageKey of Object.keys(PAGES)) {
    const page = PAGES[pageKey];
    const pagePairs = pairs.filter((p) => p.pages.includes(pageKey));
    const status = embedded[pageKey] ? "" : "  (NOT embedded — see warning above)";
    const bySection = new Map();
    for (const p of pagePairs) {
      if (!bySection.has(p.section)) bySection.set(p.section, []);
      bySection.get(p.section).push(p);
    }
    console.log(
      `\n  ${page} — ${pagePairs.length} examples in ${bySection.size} sections${status}`,
    );
    for (const section of orderedSections(new Set(bySection.keys()))) {
      const names = bySection
        .get(section)
        .map((p) => path.basename(p.title, ".ny"))
        .sort();
      console.log(`    ${section} (${names.length}): ${names.join(", ")}`);
    }
  }

  console.log("\nper-example placement (page  §section  #anchor):");
  for (const p of pairs) {
    console.log(`  ${p.title}`);
    for (const pageKey of p.pages) {
      console.log(
        `    → ${PAGES[pageKey]}  §${p.section}  #${anchorFor(pageKey, p)}`,
      );
    }
  }
}

function main() {
  const pairs = collectPairs();
  const outputs = loadOutputs();

  const embedded = {};
  for (const pageKey of Object.keys(PAGES)) {
    const pagePairs = pairs.filter((p) => p.pages.includes(pageKey));
    const markup = sectionsHtml(pageKey, pagePairs, outputs);
    const gallery = galleryHtml(INTROS[pageKey] ?? "", markup);
    embedded[pageKey] = embedGallery(
      path.join(WEBDOCS, PAGES[pageKey]),
      gallery,
      PAGES[pageKey],
    );
  }

  const embeddedPages = Object.keys(embedded)
    .filter((k) => embedded[k])
    .map((k) => PAGES[k]);
  console.log(
    `builtin snippets: ${pairs.length} examples → ${embeddedPages.join(" + ")}`,
  );
  printReport(pairs, embedded);
}

main();
