# Shared macOS / Windows CI core — subset of make test-all-core (no compiletest/fuzz/cross).

.PHONY: test-platform-core

test-platform-core: build-workspace ensure-nyra
	$(call log_step,platform core Nyra tests)
	@$(MAKE) test-cargo-workspace
	@$(MAKE) test-conformance
	@$(MAKE) test-nyra-lang
	@$(MAKE) test-optional-types
	@$(MAKE) smoke-stdlib
	@$(MAKE) smoke-stdlib-runtime
	@$(MAKE) smoke-stdlib-priority
	@$(MAKE) test-runtime-smoke
	$(call log_ok,platform core Nyra tests)
