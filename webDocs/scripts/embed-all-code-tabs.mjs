#!/usr/bin/env node
/**
 * Embed Without types / With types tabs on every Nyra code block in webDocs HTML.
 */
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { escapeHtml, tabBlock } from "./lib/code-tabs.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEBDOCS = path.resolve(__dirname, "..");
const ROOT = path.resolve(WEBDOCS, "..");
const SNIPPET_PY = path.join(ROOT, "scripts", "snippet-types.py");

const START = "<!-- NYRA_SNIPPET_START -->";
const END = "<!-- NYRA_SNIPPET_END -->";
const PRE_CODE_RE = /<pre><code>([\s\S]*?)<\/code><\/pre>/g;
const SKIP_PAGES = new Set(["stdlib.html"]);

function maskCodeTabs(html) {
  const slots = [];
  let search = 0;
  while (search < html.length) {
    const idx = html.indexOf('<div class="code-tabs"', search);
    if (idx === -1) break;
    let depth = 0;
    let j = idx;
    while (j < html.length) {
      if (html.startsWith("<div", j)) depth += 1;
      if (html.startsWith("</div>", j)) {
        depth -= 1;
        if (depth === 0) {
          j += "</div>".length;
          slots.push(html.slice(idx, j));
          const token = `<!--TABMASK${slots.length - 1}-->`;
          html = html.slice(0, idx) + token + html.slice(j);
          search = idx + token.length;
          break;
        }
      }
      j += 1;
    }
    if (j >= html.length) break;
  }
  return { html, slots };
}

function unmaskCodeTabs(html, slots) {
  slots.forEach((slot, i) => {
    html = html.replace(`<!--TABMASK${i}-->`, slot);
  });
  return html;
}

function maskBetween(html, startMarker, endMarker) {
  const slots = [];
  let s = 0;
  while (true) {
    const a = html.indexOf(startMarker, s);
    if (a === -1) break;
    const b = html.indexOf(endMarker, a);
    if (b === -1) break;
    const end = b + endMarker.length;
    slots.push(html.slice(a, end));
    const token = `<!--RANGE${slots.length - 1}-->`;
    html = html.slice(0, a) + token + html.slice(end);
    s = a + token.length;
  }
  return { html, slots };
}

function unmaskRanges(html, slots) {
  slots.forEach((slot, i) => {
    html = html.replace(`<!--RANGE${i}-->`, slot);
  });
  return html;
}

function transform(mode, code) {
  return execFileSync("python3", [SNIPPET_PY, mode, "-"], {
    input: code,
    encoding: "utf8",
  }).trimEnd();
}

function isNyraSnippet(text) {
  const t = text.trim();
  if (!t) return false;
  if (/^(nyra |cargo |curl |mkdir |export |source |wasmtime |clang |\$ |#>|# )/m.test(t)) {
    return false;
  }
  if (/^(error|warning|help):/m.test(t)) return false;
  if (/^\s*define\s+/.test(t) || /^\s*%/.test(t)) return false;
  if (/^\s*\{[\s\S]*"[\w]+"\s*:/.test(t)) return false;
  if (/\bfn\s+/.test(t)) return true;
  if (/\blet\s+(mut\s+)?\w+/.test(t)) return true;
  if (/\bstruct\s+\w+/.test(t)) return true;
  if (/\benum\s+\w+/.test(t)) return true;
  if (/\bimport\s+"/.test(t)) return true;
  if (/\bextern\s+fn\b/.test(t)) return true;
  if (/\bimpl\s+/.test(t)) return true;
  if (/\bmatch\s+/.test(t)) return true;
  if (/\bfor\s+\w+\s+in\b/.test(t)) return true;
  if (/\bconst\s+\w+/.test(t)) return true;
  return false;
}

function unwrapEntity(text) {
  return text
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&amp;/g, "&");
}

function stripExistingSnippets(html) {
  html = html.replace(
    new RegExp(
      `${START}[\\s\\S]*?<div class="code-panel active"[^>]*><pre><code>([\\s\\S]*?)</code></pre></div>[\\s\\S]*?${END}\\n?`,
      "g",
    ),
    (_m, code) => `<pre><code>${code}</code></pre>`,
  );
  return html.replace(
    new RegExp(`${START}[\\s\\S]*?${END}\\n?`, "g"),
    "",
  );
}

function embedSnippetTabs(html) {
  let count = 0;
  html = stripExistingSnippets(html);
  const r1 = maskBetween(html, "<!-- BUILTIN_CODE_TABS_START -->", "<!-- BUILTIN_CODE_TABS_END -->");
  html = r1.html;
  const r2 = maskCodeTabs(html);
  html = r2.html;

  html = html.replace(PRE_CODE_RE, (match, raw) => {
    const plain = unwrapEntity(raw.replace(/<[^>]+>/g, ""));
    if (!isNyraSnippet(plain)) return match;
    let easy;
    let typed;
    try {
      easy = transform("strip", plain);
      typed = transform("add", plain);
    } catch {
      return match;
    }
    count += 1;
    const pair = { plain: easy, typed };
    const block = tabBlock(pair);
    return `${START}\n${block}\n${END}`;
  });

  html = unmaskCodeTabs(html, r2.slots);
  html = unmaskRanges(html, r1.slots);
  return { html, count };
}

function main() {
  let total = 0;
  const pages = fs
    .readdirSync(WEBDOCS)
    .filter((n) => n.endsWith(".html"))
    .map((n) => path.join(WEBDOCS, n));

  for (const pagePath of pages) {
    if (SKIP_PAGES.has(path.basename(pagePath))) continue;
    const before = fs.readFileSync(pagePath, "utf8");
    const { html, count } = embedSnippetTabs(before);
    if (count > 0) {
      fs.writeFileSync(pagePath, html);
      total += count;
      console.log(`${path.basename(pagePath)}: ${count} snippet tab(s)`);
    }
  }
  console.log(`embed-all-code-tabs: ${total} snippet tab(s) total`);
}

main();
