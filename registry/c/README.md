# Nyra C library registry

Manifests for `nyra pkg add <name>` / `nyra pkg c add <name>`.

Each `*.toml` describes how to install a system C library and generate Nyra FFI bindings.

```toml
name = "gsl"
description = "GNU Scientific Library"
headers = ["gsl/gsl_sf.h"]
libs = ["gsl"]
pkg_config = "gsl"
brew = "gsl"
apt = "libgsl-dev"
pacman = "gsl"
dnf = "gsl-devel"
aliases = ["gnu-gsl"]
```

Header-only libs that live on GitHub (e.g. raygui):

```toml
name = "raygui"
headers = ["src/raygui.h"]
libs = ["raylib"]
brew = "raylib"
git = "https://github.com/raysan5/raygui.git"
depends = ["raylib"]
```

User overrides (optional): put extra manifests in `~/.nyra/registry/c/`.

For GitHub C projects, prefer shipping a root `nyra.toml`:

```toml
[c]
headers = ["include/cool.h"]
libraries = ["cool"]
include_dirs = ["include"]
```
