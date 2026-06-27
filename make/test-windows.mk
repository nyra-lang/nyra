# Windows CI core — Nyra language/runtime gates (subset of test-all-core).

.PHONY: test-all-windows test-all-windows-native

test-all-windows: build-workspace ensure-nyra
	$(call log_step,Windows core Nyra tests)
	@$(MAKE) test-conformance
	@$(MAKE) test-nyra-lang
	@$(MAKE) test-optional-types
	@$(MAKE) smoke-stdlib
	@$(MAKE) smoke-stdlib-runtime
	@$(MAKE) smoke-stdlib-priority
	@$(MAKE) test-runtime-smoke
	@$(MAKE) test-all-windows-native
	$(call log_ok,Windows core Nyra tests)

test-all-windows-native: ensure-nyra
	$(call log_step,native Windows build smoke)
	@$(NYRA_BIN) build $(ROOT)/examples/syntax/hello.ny --for windows -o hello_win.exe
	@hello="$(ROOT)/examples/syntax"; \
	bin=""; \
	for cand in \
	  "$$hello/target/release/hello_win.exe" \
	  "$$hello/target/debug/hello_win.exe" \
	  "$$hello/target/x86_64-pc-windows-msvc/release/hello_win.exe" \
	  "$$hello/target/x86_64-pc-windows-msvc/debug/hello_win.exe"; do \
	  if [ -f "$$cand" ]; then bin="$$cand"; break; fi; \
	done; \
	if [ -z "$$bin" ]; then echo "make: missing native Windows hello artifact" >&2; exit 1; fi; \
	printf 'make: native Windows artifact: %s\n' "$$bin"
	$(call log_ok,native Windows build smoke)
