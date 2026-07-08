# Windows CI core — platform core gates + native build smoke.

.PHONY: test-all-windows test-all-windows-native

test-all-windows test-all-windows-native: export NYRA_PROGRESS_PROFILE := platform

test-all-windows: test-platform-core
	$(call run_gate,test-all-windows-native,native Windows build smoke)
	@$(MAKE) test-platform-summary

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
