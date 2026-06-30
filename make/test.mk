# Unit, integration, conformance, and optional test gates.

.PHONY: build-workspace test-cargo-workspace test-compiletest test-count suite-clean
.PHONY: test-nyra-lang test-conformance test-optional-types test-examples-corpus
.PHONY: test-fuzz-smoke test-fuzz-nightly sync-fuzz-corpus test-fuzz-stress
.PHONY: test-sanitizer test-race-tsan test-race-native test-perf
.PHONY: test-comparison-parity test-webdocs-tabs test-abi-roundtrip test-preflight test-triage
.PHONY: update-suite-stderr

build-workspace:
	$(call log_step,cargo build --workspace)
	@cargo build --workspace
	$(call log_ok,cargo build --workspace)

test-cargo-workspace: build-workspace
	$(call log_step,cargo test --workspace)
	@cargo test --workspace -- --skip suite
	$(call log_ok,cargo test --workspace)

suite-clean:
	@$(MAKE_LIB)/suite-clean.sh

test-compiletest: suite-clean ensure-nyra
	$(call log_step,compiletest suite)
	@if ! $(MAKE_LIB)/test-count.sh >/dev/null 2>&1; then \
		printf 'make: suite count mismatch — regenerating %s profile tests\n' "$(NYRA_SUITE_PROFILE)"; \
		python3 $(MAKE_PY)/gen-suite-tests.py --profile $(NYRA_SUITE_PROFILE); \
		$(MAKE_LIB)/test-count.sh; \
	fi
	@if [ "$${NYRA_TEST_ALL:-}" = "1" ]; then \
		cargo test -p compiler suite_; \
	else \
		cargo test -p compiler suite_ -- --nocapture; \
	fi
	$(call log_ok,compiletest suite)

test-count:
	@$(MAKE_LIB)/test-count.sh

test-nyra-lang: ensure-nyra
	@$(MAKE_LIB)/nyra-lang-tests.sh

test-conformance: ensure-nyra
	@$(MAKE_LIB)/conformance-tests.sh

test-optional-types: ensure-nyra
	@$(MAKE_LIB)/test-optional-types.sh

test-examples-corpus:
	@$(MAKE_LIB)/examples-corpus.sh

test-fuzz-smoke:
	@$(MAKE_LIB)/fuzz-smoke.sh

test-fuzz-nightly:
	@$(MAKE_LIB)/fuzz-nightly.sh

sync-fuzz-corpus:
	@$(MAKE_LIB)/sync-fuzz-corpus.sh

test-fuzz-stress:
	$(call log_step,fuzz stress corpus)
	@cargo test -p compiler fuzz_stress
	$(call log_ok,fuzz stress corpus)

test-sanitizer:
	@$(MAKE_LIB)/sanitizer-check.sh

test-race-tsan:
	@$(MAKE_LIB)/race-check.sh

test-race-native:
	@$(MAKE_LIB)/race-native-check.sh

test-perf:
	@$(MAKE_LIB)/perf-check.sh

test-comparison-parity: ensure-nyra
	@$(MAKE_LIB)/check-comparison-parity.sh

test-webdocs-tabs:
	@$(MAKE_LIB)/check-webdocs-code-tabs.sh

test-abi-roundtrip: ensure-nyra
	@$(MAKE_LIB)/abi-roundtrip.sh

test-preflight:
	@$(MAKE_LIB)/test-preflight.sh

# ~5–15 min: common CI breakages in one report (target/.nyra-test-all-failures).
test-triage:
	@$(MAKE_LIB)/test-triage.sh

update-suite-stderr:
	@$(MAKE_LIB)/update-suite-stderr.sh
