# Full Nyra test suite — replaces scripts/test-all.sh
# Quick pre-check: make test-preflight
#
# Ordering: fast → slow so trivial failures surface before heavy gates
# (compiletest grid, fuzz smoke, cross-compile, optional san/perf/fuzz-nightly).
# Keeps running after a gate fails; all failures are printed at the end.

.PHONY: test-all test-all-core test-all-core-fast test-all-core-medium
.PHONY: test-all-core-heavy test-all-core-slow test-all-abi test-all-cross test-all-extended
.PHONY: test-all-summary test-all-banner

test-all: test-all-init test-all-core test-all-abi test-all-cross test-all-extended test-all-summary

test-all-init:
	@mkdir -p $(TARGET_DIR)
	@ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
		$(MAKE_LIB)/test-all-gate.sh init
	@printf 'test-all: started %s\nroot: %s\n' "$$(date '+%Y-%m-%d %H:%M:%S')" "$(ROOT)" >$(TEST_ALL_LOG)
	@ROOT="$(ROOT)" TEST_ALL_LOG="$(TEST_ALL_LOG)" \
		TEST_PERF="$(TEST_PERF)" TEST_SAN="$(TEST_SAN)" TEST_FUZZ="$(TEST_FUZZ)" \
		NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
		$(MAKE_LIB)/test-all-progress.sh init
	@printf 'make: live log: %s\n' "$(TEST_ALL_LOG)"
	@printf 'make: failures log: %s\n' "$(TEST_ALL_FAILURES_FILE)"
	@printf 'make: gate logs (on failure): %s/\n' "$(TEST_ALL_GATE_LOGS_DIR)"
	@. $(MAKE_LIB)/test-stats.sh && nyra_stats_init

test-all-core:
	$(call run_gate,build-workspace,cargo build --workspace)
	$(call run_gate,build-cli,nyra cli)
	@$(MAKE) test-all-core-fast
	@$(MAKE) test-all-core-medium
	@$(MAKE) test-all-core-heavy
	@$(MAKE) test-all-core-slow

# Seconds–~1 min: static checks, small scripts, no full compile grid.
test-all-core-fast:
	$(call log_phase,fast gates)
	$(call run_gate,test-count,suite test count)
	$(call run_gate,test-webdocs-tabs,webdocs code tabs)
	$(call run_gate,test-webdocs-snippets,webdocs snippet run)
	$(call run_gate,smoke-vscode-extension,vscode extension compile)
	$(call run_gate,test-optional-types,optional types)
	$(call run_gate,test-contrib-conformance,contrib automation CONF-CONTRIB-PY)
	$(call run_gate,test-comparison-parity,comparison parity)

# ~1–5 min: Rust unit/integration (excl. compiletest), Nyra scripts, CLI smokes.
test-all-core-medium:
	$(call log_phase,medium gates)
	$(call run_gate,test-cargo-workspace,cargo test --workspace)
	$(call run_gate,test-nyra-lang,nyra language tests)
	$(call run_gate,test-runtime-smoke,runtime smoke)
	$(call run_gate,smoke-cli,cli smoke)
	$(call run_gate,smoke-apps,apps smoke)
	$(call run_gate,smoke-sqlite,sqlite smoke)
	$(call run_gate,smoke-database,database smoke)
	$(call run_gate,smoke-serde-pkg,serde pkg smoke)

# ~5–15 min: conformance, corpus/examples, stdlib compile + runtime smokes.
test-all-core-heavy:
	$(call log_phase,heavy gates)
	$(call run_gate,test-conformance,conformance tests)
	$(call run_gate,smoke-corpus,corpus smoke)
	$(call run_gate,smoke-examples,examples smoke)
	$(call run_gate,smoke-stdlib,stdlib compile smoke)
	$(call run_gate,smoke-stdlib-priority,stdlib priority smoke)
	$(call run_gate,smoke-stdlib-medium,stdlib medium smoke)
	$(call run_gate,smoke-stdlib-runtime,stdlib runtime smoke)

# ~10+ min: compiletest grid (~3k CI / ~10k full) and libFuzzer smoke (5×60s).
test-all-core-slow:
	$(call log_phase,slow gates)
	$(call run_gate,test-compiletest,compiletest suite)
	$(call run_gate,test-fuzz-smoke,fuzz smoke)

test-all-abi:
	$(call run_gate,gen-abi-header,regenerate nyra_rt.h)
	$(call run_cmd,abi roundtrip cdylib,$(NYRA_BIN) build $(ROOT)/examples/ffi/export_greet/main.ny -o libnyra_greet --cdylib)
	$(call run_cmd,abi roundtrip rust host,cargo run --quiet --manifest-path $(ROOT)/examples/ffi/export_greet/rust_host/Cargo.toml)

test-all-cross:
	$(call run_gate,smoke-cross-wasm,cross wasm smoke)
	$(call run_gate,smoke-cross-linux,cross linux smoke)
	$(call run_gate,smoke-cross-windows,cross windows smoke)

# fuzz_stress always; optional san/perf/nightly fuzz last (longest when enabled).
test-all-extended:
	$(call run_gate,test-fuzz-stress,fuzz stress corpus)
	@if [ "$(TEST_PERF)" = "1" ]; then \
		ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
			TEST_ALL_LOG="$(TEST_ALL_LOG)" \
			NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
			$(MAKE_LIB)/test-all-gate.sh make test-perf perf check; \
	fi
	@if [ "$(TEST_SAN)" = "1" ]; then \
		ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
			TEST_ALL_LOG="$(TEST_ALL_LOG)" \
			NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
			$(MAKE_LIB)/test-all-gate.sh make test-sanitizer sanitizer check; \
		ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
			TEST_ALL_LOG="$(TEST_ALL_LOG)" \
			NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
			$(MAKE_LIB)/test-all-gate.sh make test-race-tsan race tsan; \
		ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
			TEST_ALL_LOG="$(TEST_ALL_LOG)" \
			NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
			$(MAKE_LIB)/test-all-gate.sh make test-race-native race native; \
	fi
	@if [ "$(TEST_FUZZ)" = "1" ]; then \
		ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
			TEST_ALL_LOG="$(TEST_ALL_LOG)" \
			NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
			$(MAKE_LIB)/test-all-gate.sh make test-fuzz-nightly fuzz nightly; \
	fi

test-all-summary: test-all-banner
	@ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
		TEST_ALL_LOG="$(TEST_ALL_LOG)" \
		NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
		$(MAKE_LIB)/test-all-gate.sh summary

test-all-banner:
	@. $(MAKE_LIB)/test-stats.sh && nyra_stats_read; \
	failures="$$(ROOT='$(ROOT)' TEST_ALL_FAILURES_FILE='$(TEST_ALL_FAILURES_FILE)' \
		$(MAKE_LIB)/test-all-gate.sh count)"; \
	g="\033[32m"; b="\033[1m"; dim="\033[2m"; r="\033[0m"; y="\033[33m"; red="\033[31m"; \
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
	if [ "$$failures" -gt 0 ] 2>/dev/null; then \
		printf "$${red}$${b}\n       +-------------------------------------------+\n"; \
		printf "       |  !!  %s GATE(S) FAILED — see below  !!  |\n" "$$failures"; \
		printf "       +-------------------------------------------+\n$${r}\n"; \
	else \
		printf "$${g}$${b}\n       +-------------------------------------------+\n"; \
		printf "       |     OK   A L L   T E S T S   P A S S E D   |\n"; \
		printf "       +-------------------------------------------+\n$${r}\n"; \
	fi; \
	printf "$${dim}  nyra test suite — %s$${r}\n\n" "$$($(MAKE_LIB)/test-all-progress.sh now)"
