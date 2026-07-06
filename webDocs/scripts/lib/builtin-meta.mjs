/**
 * Method labels and anchor slugs for examples/builtins/*.ny gallery (methods.html).
 */
import path from "node:path";

const MAIN_LABELS = {
  array: "Array_push / map / filter / reduce",
  json: "JSON_stringify / JSON_parse",
  math: "Math_max / min / round / random",
  string: "String helpers",
  map: "HashMap insert / get / contains",
  vec: "Vec_i32",
  strvec: "StrVec",
};

const STDLIB_STRING = {
  includes: "String_includes",
  split: "String_split",
  replace: "String_replace",
  trim: "trim",
  to_upper: "String_toUpperCase",
  to_lower: "String_toLowerCase",
};

/** Display title on methods.html — method name, not file path. */
export function methodLabel(relPlain) {
  const rel = relPlain.replace(/\\/g, "/");
  const file = path.basename(rel, ".ny");
  const folder = path.basename(path.dirname(rel));

  if (file === "math_intrinsics") {
    return "abs / min_i32 / max_i32 / clamp_i32";
  }

  if (file === "main") {
    return MAIN_LABELS[folder] ?? folder;
  }

  if (folder === "stdlib") {
    if (file.startsWith("math_")) {
      return `Math_${file.slice(5)}`;
    }
    if (file.startsWith("array_")) {
      return `Array_${file.slice(6)}`;
    }
    if (file.startsWith("string_")) {
      return STDLIB_STRING[file.slice(7)] ?? file.slice(7);
    }
    if (file === "json_parse") {
      return "JSON_parse";
    }
    if (file === "json_stringify") {
      return "JSON_stringify";
    }
    if (file === "random") {
      return "random / random_f64";
    }
    return file;
  }

  if (folder === "strings") {
    const dotMethods = {
      split: ".split()",
      trim: ".trim()",
      contains: ".contains()",
      replace: ".replace()",
      starts_with: ".starts_with()",
      ends_with: ".ends_with()",
      to_upper: ".to_upper()",
      to_lower: ".to_lower()",
      len: ".len()",
      length: ".length()",
      clone: "clone",
    };
    return dotMethods[file] ?? file;
  }

  if (folder === "arrays") {
    if (file === "sort") {
      return "sort";
    }
    if (file === "for_in") {
      return "for x in arr";
    }
    if (file === "len" || file === "length") {
      return ".len()";
    }
  }

  if (folder === "for_in") {
    if (file === "range") {
      return "for i in 0..n";
    }
    if (file === "string_chars") {
      return "for c in string";
    }
  }

  if (folder === "split_list") {
    return file === "for_in" ? "for s in split_list" : ".len() on split";
  }

  if (folder === "io") {
    return file;
  }

  if (folder === "timing") {
    return file === "time" ? "time_start / time_end" : "mem_start / mem_end";
  }

  if (folder === "date") {
    return "date()";
  }

  if (folder === "spawn") {
    return "spawn";
  }

  return file;
}

/** Anchor id for methods.html gallery links (#ex-sort). */
export function methodSlug(relPlain) {
  const rel = relPlain.replace(/\\/g, "/");
  const file = path.basename(rel, ".ny");
  const folder = path.basename(path.dirname(rel));
  if (file !== "main") {
    return file.replace(/_/g, "-");
  }
  return folder;
}

/** Paths whose stdout is non-deterministic — docs show a note instead of exact bytes. */
export const VARIABLE_OUTPUT = new Set([
  "examples/builtins/stdlib/random.ny",
  "examples/builtins/stdlib/math_random.ny",
  "examples/builtins/date/date.ny",
  "examples/builtins/timing/time.ny",
  "examples/builtins/timing/mem.ny",
]);

export function formatOutput(relPlain, raw) {
  if (VARIABLE_OUTPUT.has(relPlain.replace(/\\/g, "/"))) {
    const lines = raw.split("\n").filter(Boolean);
    const sample = lines.slice(0, 6).join("\n");
    if (relPlain.includes("random")) {
      return `(varies each run — ChaCha20 stream)\n${sample}\n…`;
    }
    if (relPlain.includes("date/")) {
      return `(local clock — values change)\n${sample}\n…`;
    }
    if (relPlain.includes("timing/")) {
      return `(timing / RSS vary by machine)\n${sample}`;
    }
  }
  return raw;
}
