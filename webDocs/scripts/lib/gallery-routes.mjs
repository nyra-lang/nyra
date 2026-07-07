/**
 * Routing config for the builtin example gallery.
 *
 * Each example (examples/builtins/<category>/<name>.ny) is placed into one or
 * more webDocs pages, grouped under a named section. This is the single source
 * of truth for "where does an example show up" — edit here to re-route.
 *
 * `methods.html` is the comprehensive reference (its hand-written table of
 * contents links into every section's anchors, so it must keep all categories).
 * `stdlib.html` is a focused view of the stdlib / free-function categories.
 */

export const PAGES = {
  methods: "methods.html",
  stdlib: "stdlib.html",
};

/**
 * category -> { section, pages }
 *   section : heading the example is grouped under
 *   pages   : which page keys (see PAGES) embed this category
 */
const CATEGORIES = {
  strings: { section: "String methods", pages: ["methods"] },
  string: { section: "String methods", pages: ["methods"] },
  arrays: { section: "Array methods", pages: ["methods"] },
  array: { section: "Array methods", pages: ["methods"] },
  vec: { section: "Vector methods", pages: ["methods"] },
  strvec: { section: "String-vector methods", pages: ["methods"] },
  map: { section: "HashMap methods", pages: ["methods"] },
  split_list: { section: "String split", pages: ["methods"] },
  for_in: { section: "For-in iteration", pages: ["methods"] },

  stdlib: { section: "Stdlib functions", pages: ["methods", "stdlib"] },
  json: { section: "JSON", pages: ["methods", "stdlib"] },
  math: { section: "Math", pages: ["methods", "stdlib"] },
  root: { section: "Math", pages: ["methods", "stdlib"] },
  io: { section: "Input / Output", pages: ["methods", "stdlib"] },
  timing: { section: "Timing & memory", pages: ["methods", "stdlib"] },
  date: { section: "Date & time", pages: ["methods", "stdlib"] },
  async: { section: "Async", pages: ["methods", "stdlib"] },
  parallel: { section: "Parallel", pages: ["methods", "stdlib"] },
  spawn: { section: "Spawn / threads", pages: ["methods", "stdlib"] },
  benchmark: { section: "Benchmark", pages: ["methods", "stdlib"] },
  defer: { section: "Defer", pages: ["methods", "stdlib"] },
};

const DEFAULT = { section: "Other", pages: ["methods"] };

/** Stable render order for sections within a page. */
export const SECTION_ORDER = [
  "String methods",
  "Array methods",
  "Vector methods",
  "String-vector methods",
  "HashMap methods",
  "String split",
  "For-in iteration",
  "Stdlib functions",
  "Math",
  "JSON",
  "Input / Output",
  "Timing & memory",
  "Date & time",
  "Async",
  "Parallel",
  "Spawn / threads",
  "Benchmark",
  "Defer",
  "Other",
];

/** Top-level category folder for an example rel path (or "root"). */
export function categoryOf(rel) {
  const norm = rel.replace(/\\/g, "/");
  const m = norm.match(/examples\/builtins\/([^/]+)\/[^/]+$/);
  return m ? m[1] : "root";
}

/** Routing decision for an example: { category, section, pages }. */
export function routeFor(rel) {
  const category = categoryOf(rel);
  const route = CATEGORIES[category] ?? DEFAULT;
  return { category, section: route.section, pages: route.pages };
}
