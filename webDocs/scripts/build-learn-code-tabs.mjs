#!/usr/bin/env node
/**
 * Inject easy / typed tabs into learn pages and examples.html where they
 * reference examples/syntax/*.ny runnable demos.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  readPair,
  stripInjectedTabs,
  tabBlock,
} from "./lib/code-tabs.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEBDOCS = path.resolve(__dirname, "..");
const ROOT = path.resolve(WEBDOCS, "..");

const LEARN_GLOB = /^learn-.*\.html$/;
const EXTRA_PAGES = ["language-basics.html", "examples.html"];

const RUN_CMD_RE =
  /<pre><code>nyra run examples\/syntax\/([\w_]+)\.ny<\/code><\/pre>/g;

const RUN_INLINE_RE =
  /<p>Try: <code>nyra run examples\/syntax\/([\w_]+)\.ny<\/code>/g;

const FILE_LABEL_RE =
  /<pre><code><span class="file-label">(examples\/syntax\/[\w_]+\.ny)<\/span>([\s\S]*?)<\/code><\/pre>/g;

function pagesToProcess() {
  return fs
    .readdirSync(WEBDOCS)
    .filter((name) => LEARN_GLOB.test(name) || EXTRA_PAGES.includes(name))
    .map((name) => path.join(WEBDOCS, name));
}

function injectRunTabs(html, root, re, formatReplacement) {
  let count = 0;
  const seen = new Set();

  html = html.replace(re, (match, stem) => {
    const rel = `examples/syntax/${stem}.ny`;
    const key = `${re.source}:${rel}`;
    if (seen.has(key)) return match;
    const pair = readPair(root, rel);
    if (!pair) return match;
    seen.add(key);
    count += 1;
    const tabs = tabBlock(pair, { wrapMarkers: true });
    return formatReplacement(tabs, rel, match);
  });

  return { html, count };
}

function injectTryItTabs(html, root) {
  html = stripInjectedTabs(html);
  let total = 0;

  const pre = injectRunTabs(html, root, RUN_CMD_RE, (tabs, rel) => {
    return `${tabs}\n<pre><code>nyra run ${rel}</code></pre>`;
  });
  html = pre.html;
  total += pre.count;

  const inline = injectRunTabs(html, root, RUN_INLINE_RE, (tabs, rel, match) => {
    return `${tabs}\n${match}`;
  });
  html = inline.html;
  total += inline.count;

  return { html, count: total };
}

function injectFileLabelTabs(html, root) {
  let count = 0;
  html = html.replace(FILE_LABEL_RE, (match, rel) => {
    const pair = readPair(root, rel);
    if (!pair) return match;
    count += 1;
    return tabBlock(pair, { wrapMarkers: true });
  });
  return { html, count };
}

function main() {
  let total = 0;
  for (const pagePath of pagesToProcess()) {
    let html = fs.readFileSync(pagePath, "utf8");
    const a = injectTryItTabs(html, ROOT);
    const b = injectFileLabelTabs(a.html, ROOT);
    if (b.html !== html) {
      fs.writeFileSync(pagePath, b.html);
      const n = a.count + b.count;
      total += n;
      console.log(`${path.basename(pagePath)}: ${n} tabbed block(s)`);
    }
  }
  console.log(`learn code tabs: ${total} block(s) embedded`);
}

main();
