# Nyra — Makefile entry point for contributors and CI.
#
# Quick start:
#   make help
#   make test-preflight    # fast smoke (~1–3 min)
#   make test-all          # full suite (same as CI core)
#   make install-dev       # build + install nyra from source

.DEFAULT_GOAL := help

include make/common.mk
include make/nyra.mk
include make/test.mk
include make/smoke.mk
include make/build.mk
include make/install.mk
include make/release.mk
include make/generators.mk
include make/test-all.mk
include make/test-platform.mk
include make/test-macos.mk
include make/test-windows.mk

.PHONY: help test build check fmt clean

help:
	@printf '%s\n' \
		'Nyra Makefile — common targets' \
		'' \
		'  make test-all          Full test suite (fast gates first, heavy last; runs all gates even on failure)' \
		'  make test-all-macos    macOS CI core (platform core + native build smoke)' \
		'  make test-all-windows  Windows CI core (platform core + native build smoke)' \
		'  make test-preflight    Fast pre-check before test-all' \
		'  make build-workspace   cargo build --workspace' \
		'  make build-cli         Build target/debug/nyra only' \
		'  make install-dev       Dev install (cargo install + stdlib sync)' \
		'  make dist              Release tarball → dist/nyra-<arch>-<os>.tar.gz (GitHub upload)' \
		'  make verify-dist       List dist/ tarball contents' \
		'  make bench             Cross-language benchmarks' \
		'  make gen-abi-header    Regenerate stdlib/nyra_rt.h' \
		'  make gen-bindings-doc  Regenerate docs/bindings.md + webDocs/bindings.html' \
		'  make build-webdocs     Regenerate webDocs search index + skill' \
		'  make sync-webdocs-code-tabs  Sync doc code-tab pairs' \
		'  make gen-suite-tests   Regenerate compiletest suite (GEN_SUITE_ARGS=--profile ci|full)' \
		'' \
		'Test subsets (test-all runs fast → slow):' \
		'  make test-all-core-fast    Count + webdocs + optional-types (~1 min)' \
		'  make test-all-core-slow    Compiletest grid + fuzz smoke (~10+ min)' \
		'  make test-all-windows  Core Nyra gates on Windows (CI subset)' \
		'  make test-conformance  CONF-LANG pass/fail/fixtures' \
		'  make test-nyra-lang    tests/nyra native suite' \
		'  make test-compiletest  tests/suite compiletest grid' \
		'  make test-abi-roundtrip ABI header + FFI cdylib roundtrip' \
		'  make test-fuzz-smoke   libFuzzer smoke (skips if no cargo-fuzz)' \
		'' \
		'Smoke:' \
		'  make smoke-cli         fmt, pkg, bind, LSP, DAP' \
		'  make smoke-stdlib      nyra check every stdlib module' \
		'  make smoke-stdlib-runtime  stdlib runtime smoke (zero-types + typed)' \
		'' \
		'Optional gates (with test-all):' \
		'  TEST_SAN=1 make test-all' \
		'  TEST_PERF=1 make test-all' \
		'  TEST_FUZZ=1 make test-all   # nightly fuzz (~25 min; enabled in CI)' \
		'  NYRA_SUITE_PROFILE=fast make test-all   # smaller compiletest grid' \
		'' \
		'Generators live in make/py/ — always invoke via make targets above.'

# Short aliases
test: test-all
build: build-workspace
check: test-preflight

clean:
	@cargo clean
	@$(MAKE) suite-clean
