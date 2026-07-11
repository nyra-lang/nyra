# Changelog

## v0.1.1 (2026-07-11)

**Bug fix — `Option<string>` drop / nullish**

- Enum payload drop always tag-checks before `free` (fixes malloc abort on `Option<string> = Option.None` + `??`).
- Regression suite: `tests/suite/run/regression/option_string_*` (zero-types / explicit / Some / generic / comptime / matrix) + conformance `pass/option/string_nullish.ny`.
- Guidelines: mandatory regression test + inference/explicit matrix for language defects (`.cursor/rules/nyra-guidelines.mdc`).

## v0.1.0 (2026-07-09)

**Stdlib gap-fill release** — ~116 builtins and stdlib helpers landed via `make gen-batch3` … `gen-batch6` contribute automation (batch3–batch6).

### Strings (builtins, no import)

- Comparison & search: `equal_fold`, `compare`, `index_byte`, `last_index_byte`, `contains`, `starts_with`, `ends_with`
- Edit & trim: `trim`, `replace`, `replacen`, `substring`, `push_char`, `pop`, `strip_ansi`, `after_sep`, `before_sep`
- Validation & transform: `is_ascii`, `collapse_ws`, `reverse`, `is_digit`, `is_alpha`, `is_alnum`, `common_prefix_len`, `pad_center`, `escape_json`, `truncate`, `split_after`

### Math (`stdlib/math.ny`)

- Rounding & classification: `floor_i32`, `ceil_i32`, `round_i32`, `trunc_i32`, `signum`, `is_nan`, `is_finite`, `is_infinite`, `fract`, `fmod`, `copysign`, `lerp`
- Integer & angles: `mod_i32`, `gcd_i32`, `lcm_i32`, `deg_to_rad`, `rad_to_deg`, `saturating_add`, `saturating_sub`, `wrapping_add`, `leading_zeros`, `trailing_zeros`, `count_ones`, `rotate_left`, `rotate_right`, `rem_euclid`

### Strconv & encoding

- Parse/format: bases, padded decimal/hex/oct/bin, `format_quote`, `f64_to_string_prec`, `parse_i64`, `parse_u64`, `parse_f32`
- `hex_encode`, `hex_encode_upper`, `hex_decode`

### Collections

- **VecI32:** `insert`, `remove`, `clear`, `sort`, `reverse`, `swap`, `extend`, `capacity`, `reserve`, `fill`, `swap_remove`, `is_empty`, `slice`, `window`, `retain`, `binary_search`
- **StrVec:** `pop`, `clear`, `reverse`, `insert`, `remove_at`, `set`, `swap`, `extend`, `is_empty`
- **HashMap:** `or_insert`, `get_or_insert`, `is_empty`, `update`; new **`HashMap_i32_i32`** with `get_or` / `get_or_insert`

### Sync & FS

- Atomics: `atomic_sub_i32`, `atomic_xor_i32`, `atomic_and_i32`, `atomic_or_i32`
- FS (`stdlib/fs/file.ny`): `file_mtime`, `rename_file`, `path_is_file`, `file_is_symlink`

### Tooling & docs

- `make gen-batch3` … `make gen-batch6`, `make batch-add-builtin BATCH=batchN`
- Examples: `examples/contrib/gap_fill_showcase.ny`, `batch6_showcase.ny` (+ typed variants)
- webDocs: `methods.html`, `nyra-skill.md`, search index updated

## v0.0.1 (2026-07-09)

**Initial release**

- Nyra programming language toolchain — compiler, CLI, stdlib, runtime, LSP, and documentation site.
- Zero-types by default with optional explicit types; LLVM-native codegen; Core + Stable Extended feature tiers.
