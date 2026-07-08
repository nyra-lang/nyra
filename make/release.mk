# GitHub Release tarball — same workflow as nyrapkg `make dist`.
#
# Usage:
#   make dist                    # dist/nyra-<arch>-<os>.tar.gz for this machine
#   make dist VERSION=1.39.0     # override workspace version label in archive
#   make dist TRIPLE=x86_64-unknown-linux-gnu   # cross-compile (needs rustup target)
#   make verify-dist
#   make clean-dist

ifndef NYRA_MAKE_RELEASE
NYRA_MAKE_RELEASE := 1

DIST_DIR := $(ROOT)/dist

VERSION ?=
ifeq ($(VERSION),)
  VERSION := $(shell sed -n '/^\[workspace.package\]/,/^\[/p' "$(ROOT)/Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)
endif

TRIPLE ?=
ifeq ($(TRIPLE),)
  TRIPLE := $(shell rustc -vV 2>/dev/null | sed -n 's/^host: //p')
endif

UNAME_S := $(shell uname -s 2>/dev/null)
UNAME_M := $(shell uname -m 2>/dev/null)

ifeq ($(UNAME_S),Darwin)
  PLATFORM := darwin
else ifeq ($(UNAME_S),Linux)
  PLATFORM := linux
else ifneq ($(findstring MINGW,$(UNAME_S)),)
  PLATFORM := windows
else ifneq ($(findstring MSYS,$(UNAME_S)),)
  PLATFORM := windows
else
  PLATFORM := unknown
endif

ifeq ($(UNAME_M),arm64)
  ARCH := aarch64
else ifeq ($(UNAME_M),aarch64)
  ARCH := aarch64
else ifeq ($(UNAME_M),x86_64)
  ARCH := x86_64
else ifeq ($(UNAME_M),amd64)
  ARCH := x86_64
else
  ARCH := $(UNAME_M)
endif

ifeq ($(PLATFORM),windows)
  ASSET := nyra-$(ARCH)-windows.zip
else
  ASSET := nyra-$(ARCH)-$(PLATFORM).tar.gz
endif

DIST := $(DIST_DIR)/$(ASSET)

.PHONY: dist release verify-dist clean-dist dist-help

dist-help:
	@printf '%s\n' \
		'Release packaging (GitHub assets):' \
		'  make dist              Build + $(ASSET)' \
		'  make release           Alias for dist' \
		'  make verify-dist       List tarball contents' \
		'  make clean-dist        Remove dist/' \
		'' \
		'Variables:' \
		"  VERSION=$(VERSION)" \
		"  TRIPLE=$(TRIPLE)" \
		"  DIST=$(DIST)"

dist:
	@test -n "$(VERSION)" || (printf 'make: error: VERSION not set (workspace Cargo.toml?)\n' >&2; exit 1)
	@test -n "$(TRIPLE)" || (printf 'make: error: TRIPLE not set (install rustc?)\n' >&2; exit 1)
	@$(MAKE_LIB)/package-release.sh "$(VERSION)" "$(TRIPLE)"
	@printf '\n✔  Wrote dist/%s\n' "$(ASSET)"
	@printf '   Upload to: https://github.com/nyra-lang/nyra/releases/new?tag=v%s\n' "$(VERSION)"
	@printf '   Asset name must be: %s\n' "$(ASSET)"

release: dist

verify-dist:
	@test -f "$(DIST)" || (printf 'make: error: missing %s — run make dist first\n' "$(DIST)" >&2; exit 1)
	@if [ "$(PLATFORM)" = "windows" ]; then \
		unzip -l "$(DIST)"; \
	else \
		tar -tzf "$(DIST)"; \
	fi
	@printf 'OK: release archive ready (%s)\n' "$(ASSET)"

clean-dist:
	@rm -rf "$(DIST_DIR)"
	@printf 'make: removed %s\n' "$(DIST_DIR)"

endif
