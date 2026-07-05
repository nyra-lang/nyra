# Code/doc generators — run via `make <target>` (implementation in make/py/).

MAKE_PY := $(ROOT)/make/py

.PHONY: gen-abi-header gen-bindings-doc gen-suite-tests gen-typed-examples
.PHONY: add-builtin remove-builtin patch-builtin
.PHONY: sync-webdocs-code-tabs gen-comparison-extended sync-comparison-typed
.PHONY: bump-comparison-hardness snippet-types strip-apps-types strip-nyra-symbol-prefix
.PHONY: gen-ar-file-index bench-comparison-html update-readme-bench

gen-abi-header:
	@python3 $(MAKE_PY)/gen-abi-header.py

gen-bindings-doc:
	@python3 $(MAKE_PY)/gen-bindings-doc.py

# Usage: make gen-suite-tests GEN_SUITE_ARGS="--profile ci|fast|full"
gen-suite-tests:
	@python3 $(MAKE_PY)/gen-suite-tests.py $(GEN_SUITE_ARGS)

gen-typed-examples:
	@python3 $(MAKE_PY)/gen-typed-examples.py

sync-webdocs-code-tabs:
	@python3 $(MAKE_PY)/sync-webdocs-code-tabs.py

gen-comparison-extended:
	@python3 $(MAKE_PY)/gen-comparison-extended.py $(GEN_COMPARISON_ARGS)

sync-comparison-typed:
	@python3 $(MAKE_PY)/sync-comparison-typed.py

bump-comparison-hardness:
	@python3 $(MAKE_PY)/bump-comparison-hardness.py $(BUMP_HARDNESS_ARGS)

snippet-types:
	@python3 $(MAKE_PY)/snippet-types.py $(SNIPPET_TYPES_ARGS)

strip-apps-types:
	@python3 $(MAKE_PY)/strip-apps-types.py $(STRIP_APPS_ARGS)

strip-nyra-symbol-prefix:
	@python3 $(MAKE_PY)/strip-nyra-symbol-prefix.py $(STRIP_PREFIX_ARGS)

gen-ar-file-index:
	@python3 $(MAKE_PY)/gen-ar-file-index.py $(GEN_AR_ARGS)

bench-comparison-html:
	@python3 $(MAKE_PY)/bench_comparison_html.py $(BENCH_HTML_ARGS)

update-readme-bench:
	@python3 $(MAKE_PY)/update-readme-bench.py

# Usage: make add-builtin                    # interactive wizard (default)
#        make add-builtin ARGS='--config make/py/builtin_dev/examples/strip_suffix.json'
#        make remove-builtin ARGS='--method strip_suffix'
#        make patch-builtin ARGS='-i'        # update existing builtin
# Docs:  make/py/builtin_dev/README.md
add-builtin:
	@python3 $(MAKE_PY)/builtin-dev.py add $(if $(ARGS),$(ARGS),-i)

remove-builtin:
	@python3 $(MAKE_PY)/builtin-dev.py remove $(if $(ARGS),$(ARGS),-i)

patch-builtin:
	@python3 $(MAKE_PY)/builtin-dev.py patch $(if $(ARGS),$(ARGS),-i)
