# macOS CI core — platform core gates + native build smoke.

.PHONY: test-all-macos test-all-macos-native

test-all-macos: test-platform-core
	$(call log_step,macOS CI)
	@$(MAKE) test-all-macos-native
	$(call log_ok,macOS CI)

test-all-macos-native: ensure-nyra
	$(call log_step,native macOS build smoke)
	@$(NYRA_BIN) build $(ROOT)/examples/syntax/hello.ny --release -o hello
	@. $(MAKE_LIB)/cross-target-helpers.sh; \
	hello="$(ROOT)/examples/syntax"; \
	bin="$$(cross_find_artifact "$$hello" release hello \
	  aarch64-apple-darwin x86_64-apple-darwin)" || { \
	  echo "make: missing native macOS hello artifact under $$hello/target/" >&2; exit 1; \
	}; \
	printf 'make: native macOS artifact: %s\n' "$$bin"; \
	"$$bin"
	$(call log_ok,native macOS build smoke)
