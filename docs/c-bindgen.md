# Nyra C Bindgen (`nyra bind c` / `nyra pkg add`)

Automatic **C header → Nyra `extern fn`** bindings via **libclang** (same AST engine as Clang/LLVM).

> Call C libraries with one command — registry install, auto-detect, or GitHub clone.

## Quick start — `nyra pkg add` (recommended)

```bash
nyra pkg init && cd myapp

nyra pkg add gsl          # registry: install + bind + nyra.mod
nyra pkg add zlib
nyra pkg add sqlite3
nyra pkg add https://github.com/org/cool-c-lib   # clone + discover + bind

nyra pkg c list           # installed in this project
nyra pkg c remove gsl
```

Same flow via the explicit alias: `nyra pkg c add gsl`.

**What happens automatically**

1. Resolve OS (macOS Homebrew / Linux apt|pacman|dnf / Windows vcpkg|MSYS2 paths)
2. Install the package if missing (interactive prompt, or `-y`)
3. Find headers (`pkg-config`, Homebrew prefix, `/usr/include`, …)
4. Run bindgen → `vendor/bindings/*.ny`
5. Patch `nyra.mod` (`link` + `link -L`)
6. Record install in `vendor/bindings/c-libs.toml`

```ny
import "vendor/bindings/gsl_sf.ny"
```

### Registry

Built-in manifests live in [`registry/c/`](../registry/c/):

`curl`, `gsl`, `libpng`, `openssl`, `raygui`, `raylib`, `sdl2`, `sqlite3`, `zlib`

Override / extend locally:

- `~/.nyra/registry/c/*.toml`
- `$NYRA_C_REGISTRY/*.toml`
- `./registry/c/*.toml`

Example manifest:

```toml
name = "gsl"
description = "GNU Scientific Library"
headers = ["gsl/gsl_sf.h"]
libs = ["gsl", "gslcblas"]
pkg_config = "gsl"
brew = "gsl"
apt = "libgsl-dev"
pacman = "gsl"
dnf = "gsl-devel"
```

### Flags

| Flag | Meaning |
|------|---------|
| `-y` / `--yes` | Install without prompting |
| `--no-install` | Never run brew/apt; fail if missing |
| `--header PATH` | Custom header |
| `-I DIR` | Extra include path |
| `--lib NAME` | Override link name(s) |
| `--path DIR` | Project root |

## Auto-detect — `nyra bind gsl`

If the library is already installed:

```bash
nyra bind gsl
nyra bind zlib --no-install
nyra bind gsl --header /custom/include/gsl/gsl_sf.h -I /custom/include
```

Searches (among others):

- **macOS:** `/opt/homebrew/include`, `/usr/local/include`, Homebrew kegs, `pkg-config`
- **Linux:** `/usr/include`, `/usr/local/include`, `pkg-config`
- **Windows:** `%VCPKG_ROOT%`, MSYS2, `C:\Program Files`

If missing, prompts:

```text
Library 'gsl' not found.

Install with: brew install gsl

Install it? [Y/n]
```

## GitHub / any git URL

```bash
nyra pkg add https://github.com/someone/cool-library
```

Nyra will:

1. `git clone --depth 1` → `vendor/c-src/<name>/`
2. Read optional root **`nyra.toml`** (best)
3. Else detect `CMakeLists.txt` / `meson.build` / `configure` / `Makefile` and try to build into `vendor/c-prefix/<name>/`
4. Find a header and run bindgen
5. Update `nyra.mod`

### Upstream `nyra.toml` (recommended for C repos)

```toml
[c]
headers = ["include/cool.h"]
libraries = ["cool"]
include_dirs = ["include"]
link_dirs = ["lib"]
```

If discovery fails:

```text
Couldn't determine how to generate bindings.

Please specify:
  Header:  include/cool.h
  Library: cool

  nyra bind c vendor/c-src/cool-library/include/cool.h --lib cool --update-mod
```

## Manual bind (any header)

```bash
nyra bind c /path/to/header.h --lib foo --update-mod
nyra pkg bind c vendor/api.h --lib mylib --update-mod
nyra bind c vendor/api.h --stdout --prefix mylib_
```

## Generated output

```ny
struct Point repr(C) {
    x: i32
    y: i32
}

extern fn make_point(x: i32, y: i32) -> Point
extern fn sqlite3_open(filename: string, ppDb: ptr) -> i32
```

## CLI reference

```text
nyra pkg add NAME|URL [-y] [--no-install] [--header PATH] [-I DIR] [--lib NAME] [--path DIR]
nyra pkg c add NAME|URL …
nyra pkg c remove NAME [--path DIR]
nyra pkg c list [--path DIR]

nyra bind NAME [-y] [--no-install] [--header PATH] [-I DIR] [--lib NAME]
nyra bind c HEADER [options]
nyra pkg bind c HEADER [options]

  --lib NAME           nyra.mod: link NAME  (repeatable)
  -I, --include DIR    clang -I path       (repeatable)
  -D, --define MACRO   clang -D            (repeatable)
  -o, --output FILE    output .ny path
  --prefix PREFIX      only functions starting with PREFIX
  --export SYM         optional shrink filter (default: all symbols)
  --shim               experimental C shims for complex signatures
  --no-shim            disable shims
  --update-mod         append link / link-source lines to nyra.mod
  --stdout             print bindings, do not write file
  --project DIR        project root (nyra bind c)
  --path DIR           project root (nyra pkg bind c)
```

## Type mapping (C → Nyra FFI)

| C | Nyra |
|---|------|
| `char` / `signed char` | `i8` |
| `unsigned char` | `u8` |
| `short` | `i16` |
| `unsigned short` | `u16` |
| `int` | `i32` |
| `unsigned` | `u32` |
| `long` / `long long` | `i64` |
| `unsigned long` / `unsigned long long` | `u64` |
| `float` / `double` | `f64` |
| `_Bool` / `bool` | `bool` |
| `const char *` | `string` |
| complete struct (ABI-safe fields) | `repr(C) struct Name { … }` |
| C enum | `i32` |
| pointers, fn pointers | `ptr` |

Unsupported signatures are skipped unless **auto shims** (`--shim`, experimental).

## Prerequisites

`brew install llvm` or `nyra toolchain install` (libclang).

## Examples

- `examples/c_raylib/` — Raylib window + game loop
- `examples/c_bindgen/` — custom C + `link-source`

## Architecture

```
registry/c/*.toml  ──┐
nyra.toml (upstream) ├──→ resolve paths / install
git URL + build      ──┘
         ↓
header.h  →  libclang AST  →  vendor/bindings/*.ny
                           →  nyra.mod link lines
                           →  vendor/bindings/c-libs.toml
```

Crate: `c-bindgen/` · CLI: `cli/src/c_lib.rs`, `cli/src/c_registry.rs`

See also [native-cc.md](native-cc.md) · [bindings.md](bindings.md) · `webDocs/c-bindgen.html`
