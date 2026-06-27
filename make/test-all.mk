# Full Nyra test suite — replaces scripts/test-all.sh
# Quick pre-check: make test-preflight

.PHONY: test-all test-all-core test-all-abi test-all-cross test-all-extended

test-all: test-all-init test-all-core test-all-abi test-all-cross test-all-extended test-all-banner
	@printf 'make: ✅ test-all completed successfully at %s\n' "$$(date '+%Y-%m-%d %H:%M:%S')"

test-all-init:
	@mkdir -p $(TARGET_DIR)
	@printf 'test-all: started %s\nroot: %s\n' "$$(date '+%Y-%m-%d %H:%M:%S')" "$(ROOT)" >$(TEST_ALL_LOG)
	@printf 'make: 🚀 Starting Nyra test suite (root: %s)\n' "$(ROOT)"
	@printf 'make: live log: %s\n' "$(TEST_ALL_LOG)"
	@. $(MAKE_LIB)/test-stats.sh && nyra_stats_init

test-all-core: build-workspace ensure-nyra
	@$(MAKE) test-cargo-workspace
	@$(MAKE) test-compiletest
	@$(MAKE) test-count
	@$(MAKE) test-fuzz-smoke
	@$(MAKE) test-nyra-lang
	@$(MAKE) smoke-apps
	@$(MAKE) smoke-sqlite
	@$(MAKE) smoke-database
	@$(MAKE) test-conformance
	@$(MAKE) test-optional-types
	@$(MAKE) smoke-stdlib
	@$(MAKE) smoke-stdlib-runtime
	@$(MAKE) smoke-stdlib-priority
	@$(MAKE) smoke-stdlib-medium
	@$(MAKE) smoke-corpus
	@$(MAKE) smoke-examples
	@$(MAKE) smoke-serde-pkg
	@$(MAKE) smoke-cli
	@$(MAKE) smoke-vscode-extension
	@$(MAKE) test-runtime-smoke
	@$(MAKE) test-webdocs-tabs
	@$(MAKE) test-comparison-parity

test-all-abi: ensure-nyra
	$(call log_step,regenerate nyra_rt.h)
	@$(MAKE) gen-abi-header
	$(call log_ok,regenerate nyra_rt.h)
	$(call log_step,abi roundtrip cdylib)
	@$(NYRA_BIN) build $(ROOT)/examples/ffi/export_greet/main.ny -o libnyra_greet --cdylib
	$(call log_ok,abi roundtrip cdylib)
	$(call log_step,abi roundtrip rust host)
	@cargo run --quiet --manifest-path $(ROOT)/examples/ffi/export_greet/rust_host/Cargo.toml
	$(call log_ok,abi roundtrip rust host)

test-all-cross: ensure-nyra
	@$(MAKE) smoke-cross-wasm
	@$(MAKE) smoke-cross-linux
	@$(MAKE) smoke-cross-windows

test-all-extended:
	@if [ "$(TEST_PERF)" = "1" ]; then $(MAKE) test-perf; fi
	@if [ "$(TEST_FUZZ)" = "1" ]; then $(MAKE) test-fuzz-nightly; fi
	@if [ "$(TEST_SAN)" = "1" ]; then \
		$(MAKE) test-sanitizer test-race-tsan test-race-native; \
	fi
	@$(MAKE) test-fuzz-stress

test-all-banner:
	@. $(MAKE_LIB)/test-stats.sh && nyra_stats_read && $(SHELL) -c '\
		g="\033[32m"; b="\033[1m"; dim="\033[2m"; r="\033[0m"; y="\033[33m"; \
		printf "\n$${g}$${b}"; \
		printf "     ███╗   ██╗██╗   ██╗██████╗  █████╗ \n"; \
		printf "     ████╗  ██║╚██╗ ██╔╝██╔══██╗██╔══██╗\n"; \
		printf "     ██╔██╗ ██║ ╚████╔╝ ██████╔╝███████║\n"; \
		printf "     ██║╚██╗██║  ╚██╔╝  ██╔══██╗██╔══██║\n"; \
		printf "     ██║ ╚████║   ██║   ██║  ██║██║  ██║\n"; \
		printf "     ╚═╝  ╚═══╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝\n"; \
		printf "$${r}\n"; \
		printf "$${y}        passed: %s   errors: %s   warnings: %s$${r}\n" \
			"$$NYRA_TEST_STATS_PASSED" "$$NYRA_TEST_STATS_ERRORS" "$$NYRA_TEST_STATS_WARNINGS"; \
		printf "$${g}$${b}\n       ╔═══════════════════════════════════════╗\n"; \
		printf "       ║     ✓   A L L   T E S T S   P A S S E D   ✓     ║\n"; \
		printf "       ╚═══════════════════════════════════════╝\n$${r}\n"; \
		printf "$${dim}  nyra test suite — %s$${r}\n\n" "$$(date +%Y-%m-%d\ %H:%M:%S)"; \
	'
