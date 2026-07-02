# Linux CI core — platform core gates + native build smoke.

.PHONY: test-all-linux test-all-linux-native

test-all-linux test-all-linux-native: export NYRA_PROGRESS_PROFILE := platform

test-all-linux: test-platform-core
	$(call run_gate,test-all-linux-native,native Linux build smoke)
	@$(MAKE) test-platform-summary

test-all-linux-native: ensure-nyra
	$(call log_step,native Linux build smoke)
	@$(NYRA_BIN) build $(ROOT)/examples/syntax/hello.ny --release -o hello
	@. $(MAKE_LIB)/cross-target-helpers.sh; \
	hello="$(ROOT)/examples/syntax"; \
	bin="$$(cross_find_artifact "$$hello" release hello \
	  x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu)" || { \
	  echo "make: missing native Linux hello artifact under $$hello/target/" >&2; exit 1; \
	}; \
	printf 'make: native Linux artifact: %s\n' "$$bin"; \
	"$$bin"
	$(call log_ok,native Linux build smoke)
