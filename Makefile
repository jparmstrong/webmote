# webmote — build & test on Linux and macOS.
#
# Linux and macOS binaries are built on dedicated machines over SSH: the working
# tree is rsync'd to the host, then `cargo` runs there. This is required because
# there is no cross-compile path for enigo's platform input backends — a macOS
# build only happens on a Mac.
#
# Local targets (build/release/local-test/fmt/check) operate on the current host.

SHELL        := /bin/bash
CARGO        := cargo

LINUX_HOST   := jp@koopa
MAC_HOST     := jp@mbp

# Where the source is synced on each remote host.
REMOTE_DIR   := ~/build/webmote

# target/ and .git are large and host-specific — never sync them.
RSYNC        := rsync -az --delete --exclude target --exclude .git

# Non-login SSH shells (notably koopa) don't have cargo on PATH; source its env
# first. Harmless where cargo is already on PATH (mbp).
REMOTE_CARGO := source $$HOME/.cargo/env 2>/dev/null; cd $(REMOTE_DIR) && cargo

.PHONY: all \
        build release local-test fmt check clean \
        linux build-linux test-linux check-linux \
        mac build-mac test-mac check-mac \
        build-all test

# Default: build + test on both platforms.
all: test

## ---- Local (current machine) -------------------------------------------------

# Fast debug build.
build:
	$(CARGO) build

# Optimised release build.
release:
	$(CARGO) build --release

# Unit/integration tests on this machine.
local-test:
	$(CARGO) test

fmt:
	$(CARGO) fmt --all

# Lint with warnings as errors (needs the clippy component).
check:
	$(CARGO) clippy -- -D warnings

clean:
	$(CARGO) clean

## ---- Linux (koopa) -----------------------------------------------------------

linux: build-linux test-linux

build-linux:
	ssh $(LINUX_HOST) 'mkdir -p $(REMOTE_DIR)'
	$(RSYNC) ./ $(LINUX_HOST):$(REMOTE_DIR)/
	ssh $(LINUX_HOST) '$(REMOTE_CARGO) build --release'

test-linux:
	ssh $(LINUX_HOST) 'mkdir -p $(REMOTE_DIR)'
	$(RSYNC) ./ $(LINUX_HOST):$(REMOTE_DIR)/
	ssh $(LINUX_HOST) '$(REMOTE_CARGO) test'

check-linux:
	ssh $(LINUX_HOST) 'mkdir -p $(REMOTE_DIR)'
	$(RSYNC) ./ $(LINUX_HOST):$(REMOTE_DIR)/
	ssh $(LINUX_HOST) '$(REMOTE_CARGO) clippy -- -D warnings'

## ---- macOS (mbp) -------------------------------------------------------------

mac: build-mac test-mac

build-mac:
	ssh $(MAC_HOST) 'mkdir -p $(REMOTE_DIR)'
	$(RSYNC) ./ $(MAC_HOST):$(REMOTE_DIR)/
	ssh $(MAC_HOST) '$(REMOTE_CARGO) build --release'

test-mac:
	ssh $(MAC_HOST) 'mkdir -p $(REMOTE_DIR)'
	$(RSYNC) ./ $(MAC_HOST):$(REMOTE_DIR)/
	ssh $(MAC_HOST) '$(REMOTE_CARGO) test'

check-mac:
	ssh $(MAC_HOST) 'mkdir -p $(REMOTE_DIR)'
	$(RSYNC) ./ $(MAC_HOST):$(REMOTE_DIR)/
	ssh $(MAC_HOST) '$(REMOTE_CARGO) clippy -- -D warnings'

## ---- Both platforms ----------------------------------------------------------

build-all: build-linux build-mac

# Build + test on both platforms (each `cargo test` compiles first).
test: test-linux test-mac
