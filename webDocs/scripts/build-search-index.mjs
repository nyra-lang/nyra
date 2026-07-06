#!/usr/bin/env node
/**
 * Build Lunr search index from webDocs HTML, sidebar nav, locales, and nyra-skill.md.
 * Indexes every page by section (h2–h4 anchors) plus navigation labels (EN + AR).
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, '..');

const SECTION_MAP = {
  index: 'Start',
  'getting-started': 'Start',
  install: 'Start',
  'learning-path': 'Start',
  'ai-skill': 'Start',
  'learn-intro': 'Learn',
  'learn-get-started': 'Learn',
  'learn-syntax': 'Learn',
  'learn-output': 'Learn',
  'learn-comments': 'Learn',
  'learn-variables': 'Learn',
  'learn-data-types': 'Learn',
  'learn-constants': 'Learn',
  'learn-operators': 'Learn',
  'learn-booleans': 'Learn',
  'learn-if-else': 'Learn',
  'learn-match': 'Learn',
  'learn-loops': 'Learn',
  'learn-while': 'Learn',
  'learn-for': 'Learn',
  'learn-functions': 'Learn',
  'learn-scope': 'Learn',
  'learn-strings': 'Learn',
  'learn-ownership': 'Learn',
  'learn-borrowing': 'Learn',
  'learn-data-structures': 'Learn',
  'learn-arrays': 'Learn',
  'learn-vectors': 'Learn',
  'learn-tuples': 'Learn',
  'learn-hashmap': 'Learn',
  'learn-structs': 'Learn',
  'learn-enums': 'Learn',
  closures: 'Learn',
  'language-basics': 'Language',
  language: 'Language',
  types: 'Language',
  reference: 'Language',
  spec: 'Language',
  generics: 'Language',
  match: 'Language',
  modules: 'Language',
  imports: 'Language',
  memory: 'Language',
  'ownership-ux': 'Language',
  async: 'Language',
  'traits-macros': 'Language',
  concurrency: 'Language',
  stdlib: 'Standard library',
  methods: 'Standard library',
  tooling: 'Toolchain',
  performance: 'Toolchain',
  pgo: 'Toolchain',
  'escape-analysis': 'Toolchain',
  diagnostics: 'Toolchain',
  'ffi-abi': 'Toolchain',
  'c-bindgen': 'Toolchain',
  bindings: 'Toolchain',
  targets: 'Toolchain',
  'editor-setup': 'Toolchain',
  packages: 'Toolchain',
  examples: 'Guides',
  'dungeon-steps': 'Guides',
  backend: 'Guides',
  'net-http': 'Guides',
  'os-hardware': 'Guides',
  integration: 'Guides',
  enterprise: 'Guides',
  'language-vs-ecosystem': 'Ecosystem',
  roadmap: 'Project',
  changelog: 'Project',
  sitemap: 'Project',
  'beginner-track': 'Learn',
  'beginner-01-first-program': 'Learn',
  'beginner-02-variables': 'Learn',
  'beginner-03-operators': 'Learn',
  'beginner-04-decisions': 'Learn',
  'beginner-05-loops': 'Learn',
  'beginner-06-functions': 'Learn',
  'beginner-07-structs-enums': 'Learn',
  'beginner-08-mini-project': 'Learn',
};

function decodeHtml(text) {
  return text
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&nbsp;/g, ' ');
}

function stripHtml(html) {
  return decodeHtml(
    html
      .replace(/<script[\s\S]*?<\/script>/gi, ' ')
      .replace(/<style[\s\S]*?<\/style>/gi, ' ')
      .replace(/<[^>]+>/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
  );
}

function slugify(text) {
  const plain = stripHtml(text)
    .toLowerCase()
    .replace(/[^\w\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-');
  return plain || 'section';
}

function loadLocale(lang) {
  const file = path.join(ROOT, 'locales', `${lang}.json`);
  if (!fs.existsSync(file)) return {};
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function getNested(obj, keyPath) {
  return keyPath.split('.').reduce((cur, part) => cur?.[part], obj);
}

function pageMeta(filePath, url) {
  const raw = fs.readFileSync(filePath, 'utf8');
  const titleMatch = raw.match(/<title>([^<]*)<\/title>/i);
  const h1Match = raw.match(/<main[^>]*>[\s\S]*?<h1[^>]*>([\s\S]*?)<\/h1>/i);
  const pageMatch = raw.match(/data-page="([^"]+)"/);
  const page = pageMatch ? pageMatch[1] : path.basename(filePath, '.html');
  const title = stripHtml((h1Match && h1Match[1]) || (titleMatch && titleMatch[1]) || url);
  return {
    page,
    title,
    url,
    section: SECTION_MAP[page] || 'Docs',
  };
}

function extractPageSections(filePath, url) {
  const raw = fs.readFileSync(filePath, 'utf8');
  const meta = pageMeta(filePath, url);
  const bodyMatch = raw.match(/<main[^>]*>([\s\S]*?)<\/main>/i);
  const mainHtml = bodyMatch ? bodyMatch[1] : raw;
  const headingRe = /<h([2-4])((?:\s[^>]*)?)>([\s\S]*?)<\/h\1>/gi;
  const headings = [];
  let match;
  while ((match = headingRe.exec(mainHtml)) !== null) {
    const attrs = match[2] || '';
    const idMatch = attrs.match(/\bid=["']([^"']*)["']/);
    headings.push({
      level: Number(match[1]),
      id: idMatch ? idMatch[1] : null,
      title: stripHtml(match[3]),
      start: match.index,
      end: match.index + match[0].length,
    });
  }

  const docs = [];
  const base = {
    pageTitle: meta.title,
    url: meta.url,
    section: meta.section,
    kind: 'content',
  };

  if (!headings.length) {
    const body = stripHtml(mainHtml);
    if (body) {
      docs.push({
        ...base,
        id: meta.url,
        title: meta.title,
        heading: '',
        body,
      });
    }
    return docs;
  }

  const intro = mainHtml.slice(0, headings[0].start);
  const introText = stripHtml(intro);
  if (introText.length > 30) {
    docs.push({
      ...base,
      id: `${meta.url}#intro`,
      url: `${meta.url}#intro`,
      title: meta.title,
      heading: meta.title,
      body: introText,
    });
  }

  for (let i = 0; i < headings.length; i += 1) {
    const h = headings[i];
    const start = h.end;
    const end = i + 1 < headings.length ? headings[i + 1].start : mainHtml.length;
    const anchor = h.id || slugify(h.title);
    const sectionUrl = `${meta.url}#${anchor}`;
    const body = stripHtml(mainHtml.slice(start, end));
    if (!body && !h.title) continue;
    docs.push({
      ...base,
      id: sectionUrl,
      url: sectionUrl,
      title: meta.title,
      heading: h.title,
      body: `${h.title} ${body}`.trim(),
    });
  }

  return docs;
}

function extractNavDocs(navPath, locales) {
  const raw = fs.readFileSync(navPath, 'utf8');
  const docs = [];
  const sections = raw.split(/<section>/i).slice(1);

  for (const block of sections) {
    const labelMatch = block.match(
      /<div class="nav-label"[^>]*>([^<]*)<\/div>/i
    );
    const navSection = labelMatch ? stripHtml(labelMatch[1]) : 'Navigation';
    const labelAttrsMatch = block.match(/<div class="nav-label"\s([^>]+)>/i);
    const labelI18nMatch = labelAttrsMatch
      ? labelAttrsMatch[1].match(/\bdata-i18n="([^"]*)"/)
      : null;
    const navSectionKey = labelI18nMatch ? labelI18nMatch[1] : '';
    const navSectionAr = navSectionKey
      ? getNested(locales.ar, navSectionKey) || ''
      : '';

    const linkRe = /<a\s+([^>]+)>([\s\S]*?)<\/a>/gi;
    let link;
    while ((link = linkRe.exec(block)) !== null) {
      const attrs = link[1];
      const hrefMatch = attrs.match(/\bhref="([^"#][^"]*)"/);
      if (!hrefMatch) continue;
      const href = hrefMatch[1];
      const i18nMatch = attrs.match(/\bdata-i18n="([^"]*)"/);
      const i18nKey = i18nMatch ? i18nMatch[1] : '';
      const label = stripHtml(link[2]);
      const labelAr = i18nKey ? getNested(locales.ar, i18nKey) || '' : '';
      const page = path.basename(href, '.html');
      const body = [label, labelAr, navSection, navSectionAr].filter(Boolean).join(' ');
      docs.push({
        id: `nav:${href}`,
        url: href,
        pageTitle: label,
        title: label,
        heading: 'Navigation',
        body,
        section: SECTION_MAP[page] || navSection,
        kind: 'nav',
      });
    }
  }

  return docs;
}

function extractMarkdownSections(filePath, url, docTitle, docSection) {
  const raw = fs.readFileSync(filePath, 'utf8');
  const docs = [];
  const parts = raw.split(/^##\s+/m);

  if (parts.length <= 1) {
    const body = raw
      .replace(/^#+\s/gm, '')
      .replace(/```[\s\S]*?```/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
    docs.push({
      id: url,
      url,
      pageTitle: docTitle,
      title: docTitle,
      heading: '',
      body,
      section: docSection,
      kind: 'content',
    });
    return docs;
  }

  const intro = parts[0]
    .replace(/^#\s+[^\n]+\n?/, '')
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  if (intro.length > 30) {
    docs.push({
      id: `${url}#intro`,
      url: `${url}#intro`,
      pageTitle: docTitle,
      title: docTitle,
      heading: docTitle,
      body: intro,
      section: docSection,
      kind: 'content',
    });
  }

  for (const part of parts.slice(1)) {
    const nl = part.indexOf('\n');
    const heading = nl >= 0 ? part.slice(0, nl).trim() : part.trim();
    const rest = nl >= 0 ? part.slice(nl + 1) : '';
    const anchor = slugify(heading);
    const body = rest
      .replace(/```[\s\S]*?```/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
    docs.push({
      id: `${url}#${anchor}`,
      url: `${url}#${anchor}`,
      pageTitle: docTitle,
      title: docTitle,
      heading,
      body: `${heading} ${body}`.trim(),
      section: docSection,
      kind: 'content',
    });
  }

  return docs;
}

const locales = { en: loadLocale('en'), ar: loadLocale('ar') };
const docs = [];

for (const name of fs.readdirSync(ROOT)) {
  if (!name.endsWith('.html') || name.startsWith('_')) continue;
  docs.push(...extractPageSections(path.join(ROOT, name), name));
}

const navPath = path.join(ROOT, '_includes', 'sidebar-nav.html');
if (fs.existsSync(navPath)) {
  docs.push(...extractNavDocs(navPath, locales));
}

const skillPath = path.join(ROOT, 'nyra-skill.md');
if (fs.existsSync(skillPath)) {
  docs.push(
    ...extractMarkdownSections(
      skillPath,
      'nyra-skill.md',
      'Nyra AI Skill (nyra-skill.md)',
      'AI'
    )
  );
}

const seen = new Set();
const unique = docs.filter((d) => {
  if (seen.has(d.id)) return false;
  seen.add(d.id);
  return d.body.length > 0 || d.kind === 'nav';
});

const out = { generated: new Date().toISOString(), docs: unique };
fs.writeFileSync(path.join(ROOT, 'search-index.json'), JSON.stringify(out, null, 2));
console.log(`search-index.json: ${unique.length} entries (${fs.readdirSync(ROOT).filter((n) => n.endsWith('.html')).length} HTML pages)`);
