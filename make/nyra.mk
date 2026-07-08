# Nyra CLI binary — build once, reuse across smoke/test targets.
# Also build TLS staticlibs so HTTPS conformance (rustls + native) can link in CI.
$(NYRA_BIN):
	@printf 'make: building nyra cli + TLS runtimes...\n'
	@cargo build -q -p cli -p nyra-rt-tls -p nyra-rt-tls-native

.PHONY: build-cli build-compiler-ffi ensure-nyra
build-compiler-ffi:
	@cargo build -q -p compiler-ffi
build-cli: $(NYRA_BIN)
ensure-nyra: build-cli build-compiler-ffi
