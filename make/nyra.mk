# Nyra CLI binary — build once, reuse across smoke/test targets.
# TLS staticlibs must match the MinGW link triple on Windows (not MSVC host).
$(NYRA_BIN):
	@printf 'make: building nyra cli + TLS runtimes...\n'
	@bash "$(MAKE_LIB)/build-cli-tls.sh"

.PHONY: build-cli build-compiler-ffi ensure-nyra
build-compiler-ffi:
	@cargo build -q -p compiler-ffi
build-cli: $(NYRA_BIN)
ensure-nyra: build-cli build-compiler-ffi
