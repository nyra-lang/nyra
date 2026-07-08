# Nyra CLI binary — build once, reuse across smoke/test targets.
# Also build nyra-rt-tls so HTTPS/TLS conformance can link libnyra_rt_tls.a in CI.
$(NYRA_BIN):
	@printf 'make: building nyra cli + nyra-rt-tls...\n'
	@cargo build -q -p cli -p nyra-rt-tls

.PHONY: build-cli build-compiler-ffi ensure-nyra
build-compiler-ffi:
	@cargo build -q -p compiler-ffi
build-cli: $(NYRA_BIN)
ensure-nyra: build-cli build-compiler-ffi
