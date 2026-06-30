# Smoke tests — stdlib, examples, CLI, apps, cross-compile.

.PHONY: smoke-cli smoke-apps smoke-database smoke-sqlite
.PHONY: smoke-stdlib smoke-stdlib-runtime smoke-stdlib-priority smoke-stdlib-medium
.PHONY: smoke-corpus smoke-examples smoke-serde-pkg smoke-cross
.PHONY: smoke-vscode-extension smoke-runtime smoke-cross-wasm smoke-cross-linux smoke-cross-windows
.PHONY: test-runtime-smoke

smoke-cli: ensure-nyra
	@$(MAKE_LIB)/cli-smoke.sh

smoke-apps: ensure-nyra
	@$(MAKE_LIB)/apps-smoke.sh

smoke-database: ensure-nyra
	@$(MAKE_LIB)/database-smoke.sh

smoke-sqlite: ensure-nyra
	@$(MAKE_LIB)/sqlite-smoke.sh

smoke-stdlib: ensure-nyra
	@$(MAKE_LIB)/stdlib-smoke.sh

smoke-stdlib-runtime: ensure-nyra
	@$(MAKE_LIB)/stdlib-runtime-smoke.sh

smoke-stdlib-priority: ensure-nyra
	@$(MAKE_LIB)/stdlib-priority-smoke.sh

smoke-stdlib-medium: ensure-nyra
	@$(MAKE_LIB)/stdlib-medium-smoke.sh

smoke-corpus: ensure-nyra
	@$(MAKE_LIB)/corpus-smoke.sh

smoke-examples: ensure-nyra
	@$(MAKE_LIB)/example-smoke.sh

smoke-serde-pkg: ensure-nyra
	@$(MAKE_LIB)/serde-pkg-smoke.sh

smoke-cross: ensure-nyra
	@$(MAKE_LIB)/cross-smoke.sh

smoke-vscode-extension:
	$(call log_step,vscode extension compile)
	@cd $(ROOT)/extensions/nyra && npm install --silent && npm run compile
	$(call log_ok,vscode extension compile)

# Nyra run/test smoke steps from the legacy test-all.sh.
test-runtime-smoke: ensure-nyra
	@$(MAKE_LIB)/runtime-smoke.sh

smoke-cross-wasm: ensure-nyra
	@. $(MAKE_LIB)/wasm-toolchain.sh; \
	if wasm_toolchain_ready; then \
		$(NYRA_BIN) build $(ROOT)/examples/syntax/hello.ny --for wasm -o hello.wasm; \
		wasm_bin="$$(cd $(ROOT)/examples/syntax && pwd)/target/wasm32-wasip1/debug/hello.wasm"; \
		test -f "$$wasm_bin"; \
		if command -v wasmtime >/dev/null 2>&1; then wasmtime "$$wasm_bin"; \
		else printf 'make: note: wasmtime not installed; skipping wasm run\n'; fi; \
	else printf 'make: %s\n' "$$(wasm_toolchain_hint)"; \
	fi

smoke-cross-linux: ensure-nyra
	@. $(MAKE_LIB)/cross-target-helpers.sh; \
	hello="$(ROOT)/examples/syntax"; \
	if cross_linux_linker_ready; then \
		$(NYRA_BIN) build $(ROOT)/examples/syntax/hello.ny --release --for linux; \
		bin="$$(cross_find_artifact "$$hello" release hello)" || { \
			echo "make: missing linux artifact under $$hello/target/" >&2; exit 1; \
		}; \
		printf 'make: linux artifact: %s\n' "$$bin"; \
		if [ -x "$$bin" ]; then "$$bin"; fi; \
	else cross_log_skip "linux cross linker unavailable"; fi

smoke-cross-windows: ensure-nyra
	@. $(MAKE_LIB)/cross-target-helpers.sh; \
	hello="$(ROOT)/examples/syntax"; \
	if cross_windows_linker_ready; then \
		$(NYRA_BIN) build $(ROOT)/examples/syntax/spawn_channel.ny --for windows -o spawn_win.exe; \
		bin="$$(cross_find_artifact "$$hello" debug spawn_win.exe)" || { \
			echo "make: missing windows cross artifact under $$hello/target/" >&2; exit 1; \
		}; \
		printf 'make: windows cross artifact: %s\n' "$$bin"; \
	else cross_log_skip "mingw-w64 not installed (apt: g++-mingw-w64)"; fi
