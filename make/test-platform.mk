# Shared macOS / Windows CI core — same run_gate aggregation as make test-all.

.PHONY: test-platform-core test-platform-init test-platform-summary

# Progress bar gate count + auto-init profile for platform CI only.
test-platform-init test-platform-core test-platform-summary: export NYRA_PROGRESS_PROFILE := platform

test-platform-init:
	@mkdir -p $(TARGET_DIR)
	@ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
		$(MAKE_LIB)/test-all-gate.sh init
	@printf 'test-platform: started %s\nroot: %s\n' "$$(date '+%Y-%m-%d %H:%M:%S')" "$(ROOT)" >$(TEST_ALL_LOG)
	@ROOT="$(ROOT)" TEST_ALL_LOG="$(TEST_ALL_LOG)" \
		NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
		NYRA_PROGRESS_PROFILE=platform \
		$(MAKE_LIB)/test-all-progress.sh init
	@printf 'make: live log: %s\n' "$(TEST_ALL_LOG)"
	@printf 'make: failures log: %s\n' "$(TEST_ALL_FAILURES_FILE)"
	@printf 'make: gate logs (on failure): %s/\n' "$(TEST_ALL_GATE_LOGS_DIR)"
	@. $(MAKE_LIB)/test-stats.sh && nyra_stats_init

test-platform-core: test-platform-init
	$(call run_gate,build-workspace,cargo build --workspace)
	$(call run_gate,test-cargo-workspace,cargo test --workspace)
	$(call run_gate,test-conformance,conformance tests)
	$(call run_gate,test-nyra-lang,nyra language tests)
	$(call run_gate,test-optional-types,optional types)
	$(call run_gate,smoke-stdlib,stdlib compile smoke)
	$(call run_gate,smoke-stdlib-runtime,stdlib runtime smoke)
	$(call run_gate,smoke-stdlib-priority,stdlib priority smoke)
	$(call run_gate,test-runtime-smoke,runtime smoke)

test-platform-summary:
	@ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
		TEST_ALL_LOG="$(TEST_ALL_LOG)" \
		NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
		$(MAKE_LIB)/test-all-gate.sh summary
