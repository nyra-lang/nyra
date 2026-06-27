# Shared variables for Nyra Make targets.
ifndef NYRA_MAKE_COMMON
NYRA_MAKE_COMMON := 1

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
MAKE_LIB := $(ROOT)/make/lib
MAKE_PY := $(ROOT)/make/py
TARGET_DIR := $(ROOT)/target
NYRA_BIN := $(TARGET_DIR)/debug/nyra
TEST_ALL_LOG := $(TARGET_DIR)/test-all.txt
NYRA_TEST_STATS_FILE ?= $(TARGET_DIR)/.nyra-test-all-stats

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
	@printf 'make: ⏳ %s ...\n' "$(1)"
endef

define log_ok
	@printf 'make: ✅ ok — %s\n' "$(1)"
endef

endif
