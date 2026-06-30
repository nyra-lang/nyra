# CI stage tiers for .github/workflows/ci.yml (fast → slow, parallel within each tier).
# Monolithic local/CI entry points (test-all-macos / test-all-windows) stay unchanged.

.PHONY: test-platform-ci-build test-platform-ci-summary
.PHONY: test-platform-ci-tier1 test-platform-ci-tier2 test-platform-ci-tier3

# Stage 0 — shared workspace + nyra binary (one job per OS before matrix tiers).
test-platform-ci-build:
	$(call log_step,CI build workspace + cli)
	@$(MAKE) build-workspace build-cli
	$(call log_ok,CI build workspace + cli)

# Tier 1 — seconds to ~3 min (matrix: optional-types, conformance, cargo-workspace).
test-platform-ci-tier1: ensure-nyra
	$(call log_step,CI tier1 $(NYRA_CI_GATE))
	@$(MAKE) $(NYRA_CI_GATE)
	$(call log_ok,CI tier1 $(NYRA_CI_GATE))

# Tier 2 — ~3–10 min (matrix: nyra-lang, stdlib-priority).
test-platform-ci-tier2: ensure-nyra
	$(call log_step,CI tier2 $(NYRA_CI_GATE))
	@$(MAKE) $(NYRA_CI_GATE)
	$(call log_ok,CI tier2 $(NYRA_CI_GATE))

# Tier 3 — ~5–20 min (matrix: stdlib compile, stdlib-runtime, runtime smoke).
test-platform-ci-tier3: ensure-nyra
	$(call log_step,CI tier3 $(NYRA_CI_GATE))
	@$(MAKE) $(NYRA_CI_GATE)
	$(call log_ok,CI tier3 $(NYRA_CI_GATE))

# Print aggregated failure log (optional local helper).
test-platform-ci-summary:
	@ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
		$(MAKE_LIB)/test-all-gate.sh summary
