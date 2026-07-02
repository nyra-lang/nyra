# Nyra CLI binary — build once, reuse across smoke/test targets.
$(NYRA_BIN):
	@printf 'make: building nyra cli...\n'
	@cargo build -q -p cli

.PHONY: build-cli build-compiler-ffi ensure-nyra
build-compiler-ffi:
	@cargo build -q -p compiler-ffi
build-cli: $(NYRA_BIN)
ensure-nyra: build-cli build-compiler-ffi
