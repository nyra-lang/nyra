# Build, package, benchmark, and documentation targets.

.PHONY: bench build-webdocs package-vscode package-release

bench: ensure-nyra
	@$(MAKE_LIB)/bench.sh $(BENCH_ARGS)

build-webdocs:
	@$(MAKE_LIB)/build-webdocs.sh

package-vscode:
	@$(MAKE_LIB)/package-vscode-extension.sh

# Usage: make package-release VERSION=0.0.1 TRIPLE=x86_64-unknown-linux-gnu
package-release:
	@test -n "$(VERSION)" || (printf 'make: error: VERSION is required\n' >&2; exit 1)
	@test -n "$(TRIPLE)" || (printf 'make: error: TRIPLE is required\n' >&2; exit 1)
	@$(MAKE_LIB)/package-release.sh "$(VERSION)" "$(TRIPLE)"
