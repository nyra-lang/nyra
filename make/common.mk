# Shared variables for Nyra Make targets.
ifndef NYRA_MAKE_COMMON
NYRA_MAKE_COMMON := 1

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
MAKE_LIB := $(ROOT)/make/lib
MAKE_PY := $(ROOT)/make/py
TARGET_DIR := $(ROOT)/target
NYRA_BIN := $(TARGET_DIR)/debug/nyra
TEST_ALL_LOG := $(TARGET_DIR)/test-all.txt
TEST_ALL_FAILURES_FILE := $(TARGET_DIR)/.nyra-test-all-failures
TEST_ALL_GATE_LOGS_DIR := $(TARGET_DIR)/.nyra-test-all-gate-logs
NYRA_TEST_ALL_PROGRESS_FILE := $(TARGET_DIR)/.nyra-test-all-progress
NYRA_TEST_STATS_FILE ?= $(TARGET_DIR)/.nyra-test-all-stats

export TEST_ALL_FAILURES_FILE
export TEST_ALL_LOG
export TEST_ALL_GATE_LOGS_DIR
export NYRA_TEST_ALL_PROGRESS_FILE

export NYRA_ROOT := $(ROOT)
export NYRA_BIN
export NYRA := $(NYRA_BIN)
export NYRA_TEST_STATS_FILE
export NYRA_SUITE_PROFILE

SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c

# Optional gates (same env vars as the legacy test-all.sh).
TEST_PERF ?=
TEST_FUZZ ?=
TEST_SAN ?=
NYRA_SUITE_PROFILE ?= ci

define log_step
	@if [ "$${NYRA_TEST_ALL:-}" != "1" ]; then \
		printf 'make: >> %s ...\n' "$(1)"; \
	fi
endef

define log_ok
	@if [ "$${NYRA_TEST_ALL:-}" != "1" ]; then \
		printf 'make: ok  %s\n' "$(1)"; \
	fi
endef

define log_phase
	@ROOT="$(ROOT)" TEST_ALL_LOG="$(TEST_ALL_LOG)" \
		TEST_PERF="$(TEST_PERF)" TEST_SAN="$(TEST_SAN)" TEST_FUZZ="$(TEST_FUZZ)" \
		NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
		$(MAKE_LIB)/test-all-progress.sh phase '$(1)'
endef

# test-all gates: run to completion, collect failures for the final summary.
define run_gate
	@ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
		TEST_ALL_LOG="$(TEST_ALL_LOG)" \
		NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
		$(MAKE_LIB)/test-all-gate.sh make $(1) '$(2)'
endef

define run_cmd
	@ROOT="$(ROOT)" TEST_ALL_FAILURES_FILE="$(TEST_ALL_FAILURES_FILE)" \
		TEST_ALL_LOG="$(TEST_ALL_LOG)" \
		NYRA_TEST_ALL_PROGRESS_FILE="$(NYRA_TEST_ALL_PROGRESS_FILE)" \
		$(MAKE_LIB)/test-all-gate.sh cmd '$(1)' $(2)
endef

endif
