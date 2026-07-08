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

  if (folder === "date") {
    return "date()";
  }

  if (folder === "spawn") {
    return "spawn";
  }

  if (folder === "io") {
    if (file === "print_color") return "print(..., color:)";
    return file;
  }

  if (folder === "timing") {
    return file === "time" ? "time_start / time_end" : "mem_start / mem_end";
  }

  const SUGAR_LABELS = {
    "json_sugar/jparse_jstr": "jparse / jstr / jnum / jbool",
    "json_sugar/jobj": "jobj",
    "json_sugar/obj_dict": "obj / dict / jstringify / jraw",
    "json_sugar/dict_i32": "dict_i32()",
    "json_sugar/jfield": "jfield",
    "sb/sb_build": "sb() / cat / cat3 / cat4",
    "fs_sugar/slurp_spit": "slurp / spit / spit_append / rm",
    "fs_sugar/ls_rm": "ls / rm",
    "fs_sugar/create_dir": "create_dir / remove_dir",
    "time_sugar/now_ms": "now() / ms().sleep()",
    "env/env_or": "env / env_or / env_set / env_has",
    "process_sugar/cmd_run": "cmd().arg().run() / .output()",
    "uuid/uuid_len": "uuid()",
    "encoding/b64": "b64 / b64d / url_encode",
    "error_sugar/err_show": "err_io().context()",
    "error_sugar/err_kinds": "err_json / err_invalid",
    "http_sugar/form_params": "form() / params()",
    "http_sugar/cookies_headers": "cookies() / headers()",
    "http_sugar/req_builder": "req().timeout().header()",
    "vec_sugar/vec_hofs": "vec() filter/map/reduce/contains",
    "strs_sugar/strs_hofs": "strs() / lines() / joined",
    "qb/to_sql": "qb().select().from().where().to_sql()",
    "map_sugar/keys_remove": "HashMap contains / remove / get",
    "strings/replacen": ".replacen()",
    "math/sin_cos": "sin / cos / max_f64",
  };
  const key = `${folder}/${file}`;
  if (SUGAR_LABELS[key]) return SUGAR_LABELS[key];

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
  "examples/builtins/uuid/uuid_len.ny",
  "examples/builtins/time_sugar/now_ms.ny",
  "examples/builtins/io/print_color.ny",
  "examples/builtins/fs_sugar/ls_rm.ny",
]);

export function formatOutput(relPlain, raw) {
  const rel = relPlain.replace(/\\/g, "/");
  if (VARIABLE_OUTPUT.has(rel)) {
    const lines = raw.split("\n").filter(Boolean);
    const sample = lines.slice(0, 6).join("\n");
    if (rel.includes("random")) {
      return `(varies each run — ChaCha20 stream)\n${sample}\n…`;
    }
    if (rel.includes("date/")) {
      return `(local clock — values change)\n${sample}\n…`;
    }
    if (rel.includes("timing/")) {
      return `(timing / RSS vary by machine)\n${sample}`;
    }
    if (rel.includes("uuid/")) {
      return `(UUID string — length is always 36)\n36`;
    }
    if (rel.includes("now_ms")) {
      return `(elapsed ≥ 0 after ms(1).sleep())\n1`;
    }
    if (rel.includes("print_color")) {
      return `(ANSI green on supporting terminals)\nok`;
    }
    if (rel.includes("ls_rm")) {
      return `(directory entry count / rm status)\n${sample}`;
    }
  }
  return raw;
}
