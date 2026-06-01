# recon — Makefile
#
# Run `make` or `make help` for the target list.
# Recipes use tabs (required by make).

BIN          := recon
RELEASE_BIN  := target/release/$(BIN)
DEBUG_BIN    := target/debug/$(BIN)
MANUAL_MD    := docs/MANUAL.md
MANUAL_PDF   := docs/MANUAL.pdf

CARGO        ?= cargo

# `impersonate` Cargo feature: opt-in TLS+H2 browser fingerprint impersonation
# via rquest (BoringSSL) + rquest_util. Off by default — enable via the
# *-impersonate targets below or by passing FEATURES="--features impersonate"
# to any standard target.
IMPERSONATE   := --features impersonate
FEATURES     ?=

.DEFAULT_GOAL := help

.PHONY: help build release all check test test-quiet fmt fmt-check clippy lint \
        doc run install uninstall clean clean-all distclean size pdf \
        flags examples bump-check ci \
        build-impersonate release-impersonate all-impersonate \
        check-impersonate test-impersonate run-impersonate \
        install-impersonate ci-impersonate \
        linux-deps linux tarball deb dist dist-clean-deb

help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make \033[36m<target>\033[0m\n\nTargets:\n"} \
	     /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# ---------- build ----------

build: ## Debug build
	$(CARGO) build $(FEATURES)

release: ## Release build (optimized)
	$(CARGO) build --release $(FEATURES)

all: build release ## Build both debug and release (per project convention)

build-impersonate: ## Debug build with TLS browser-fingerprint impersonation (BoringSSL; first build is slow)
	$(CARGO) build $(IMPERSONATE)

release-impersonate: ## Release build with TLS browser-fingerprint impersonation
	$(CARGO) build --release $(IMPERSONATE)

all-impersonate: build-impersonate release-impersonate ## Both builds with --features impersonate

check: ## Fast type-check without producing a binary
	$(CARGO) check --all-targets $(FEATURES)

check-impersonate: ## Fast type-check with --features impersonate
	$(CARGO) check --all-targets $(IMPERSONATE)

# ---------- test / lint ----------

test: ## Run all tests
	$(CARGO) test $(FEATURES)

test-impersonate: ## Run all tests with --features impersonate
	$(CARGO) test $(IMPERSONATE)

test-quiet: ## Run tests with minimal output
	$(CARGO) test --quiet $(FEATURES)

fmt: ## Format the codebase
	$(CARGO) fmt --all

fmt-check: ## Verify formatting without modifying files
	$(CARGO) fmt --all -- --check

clippy: ## Run clippy with warnings denied
	$(CARGO) clippy --all-targets -- -D warnings

lint: fmt-check clippy ## fmt-check + clippy

# ---------- run / install ----------

run: ## Run the debug binary (use ARGS="..." to pass arguments)
	$(CARGO) run $(FEATURES) -- $(ARGS)

run-impersonate: ## Run the impersonate-feature debug binary (use ARGS="..." to pass arguments)
	$(CARGO) run $(IMPERSONATE) -- $(ARGS)

install: release ## Install the release binary into ~/.cargo/bin
	$(CARGO) install --path . --force $(FEATURES)

install-impersonate: release-impersonate ## Install recon with --features impersonate into ~/.cargo/bin
	$(CARGO) install --path . --force $(IMPERSONATE)

uninstall: ## Remove the installed binary
	$(CARGO) uninstall $(BIN) || true

# ---------- docs ----------

doc: ## Build rustdoc for this crate (no deps)
	$(CARGO) doc --no-deps

pdf: release ## Regenerate docs/MANUAL.pdf from MANUAL.md (requires agent-browser on PATH)
	$(RELEASE_BIN) --md-to-pdf $(MANUAL_MD) \
	    --toc --toc-depth 3 --gfm \
	    --unsafe-html --page-break-on-h1 \
	    --doc-title 'recon User Manual' \
	    -o $(MANUAL_PDF)

flags: release ## Print recon --flags (useful for the per-flag checklist)
	$(RELEASE_BIN) --flags

examples: release ## Print recon --examples
	$(RELEASE_BIN) --examples

# ---------- cleaning ----------

clean: ## Remove the entire target/ directory (use this to reclaim disk)
	$(CARGO) clean

clean-all: clean ## clean + remove generated rustdoc, dist/, and stray artefacts
	rm -rf target/doc dump.rdb $(DIST)

distclean: clean-all ## clean-all + drop Cargo.lock (rarely needed)
	rm -f Cargo.lock

# ---------- linux cross-build / debian packaging ----------
# Cross-compiles the DEFAULT recon build (includes ssh) from macOS to Linux
# via cargo-zigbuild, then packages .deb via cargo-deb. No impersonate variant.
# `bundled-sqlite` vendors sqlite (zig's Linux sysroot has no libsqlite3).
# Requires zig, cargo-zigbuild, cargo-deb, and the two rustup targets
# (run `make linux-deps`). See README "Building Debian packages".

DIST        := dist
LINUX_AMD64 := x86_64-unknown-linux-gnu
LINUX_ARM64 := aarch64-unknown-linux-gnu
LINUX_AMD64_MUSL := x86_64-unknown-linux-musl
LINUX_ARM64_MUSL := aarch64-unknown-linux-musl
GLIBC       := 2.28
VERSION     := $(shell grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')

linux-deps: ## Check Linux cross-build prerequisites; print install hints if missing
	@command -v zig >/dev/null 2>&1 || { echo "missing: zig            — brew install zig"; exit 1; }
	@command -v cargo-zigbuild >/dev/null 2>&1 || { echo "missing: cargo-zigbuild — cargo install cargo-zigbuild"; exit 1; }
	@command -v cargo-deb >/dev/null 2>&1 || { echo "missing: cargo-deb      — cargo install cargo-deb"; exit 1; }
	@rustup target list --installed | grep -q '^$(LINUX_AMD64)$$' || { echo "missing rustup target: $(LINUX_AMD64) — rustup target add $(LINUX_AMD64)"; exit 1; }
	@rustup target list --installed | grep -q '^$(LINUX_ARM64)$$' || { echo "missing rustup target: $(LINUX_ARM64) — rustup target add $(LINUX_ARM64)"; exit 1; }
	@rustup target list --installed | grep -q '^$(LINUX_AMD64_MUSL)$$' || { echo "missing rustup target: $(LINUX_AMD64_MUSL) — rustup target add $(LINUX_AMD64_MUSL)"; exit 1; }
	@rustup target list --installed | grep -q '^$(LINUX_ARM64_MUSL)$$' || { echo "missing rustup target: $(LINUX_ARM64_MUSL) — rustup target add $(LINUX_ARM64_MUSL)"; exit 1; }
	@echo "linux cross-build prerequisites OK"

linux: linux-deps ## Cross-build the release binary for amd64 + arm64, glibc + musl
	$(CARGO) zigbuild --release --features bundled-sqlite --target $(LINUX_AMD64).$(GLIBC)
	$(CARGO) zigbuild --release --features bundled-sqlite --target $(LINUX_ARM64).$(GLIBC)
	$(CARGO) zigbuild --release --features bundled-sqlite --target $(LINUX_AMD64_MUSL)
	$(CARGO) zigbuild --release --features bundled-sqlite --target $(LINUX_ARM64_MUSL)

tarball: linux ## Package each Linux binary + LICENSE + README into dist/*.tar.gz
	mkdir -p $(DIST)
	tar -czf $(DIST)/recon-$(VERSION)-x86_64-linux.tar.gz  -C target/$(LINUX_AMD64)/release recon -C $(CURDIR) LICENSE README.md
	tar -czf $(DIST)/recon-$(VERSION)-aarch64-linux.tar.gz -C target/$(LINUX_ARM64)/release recon -C $(CURDIR) LICENSE README.md
	tar -czf $(DIST)/recon-$(VERSION)-x86_64-linux-musl.tar.gz  -C target/$(LINUX_AMD64_MUSL)/release recon -C $(CURDIR) LICENSE README.md
	tar -czf $(DIST)/recon-$(VERSION)-aarch64-linux-musl.tar.gz -C target/$(LINUX_ARM64_MUSL)/release recon -C $(CURDIR) LICENSE README.md

deb: linux ## Build .deb packages for amd64 + arm64 into dist/
	mkdir -p $(DIST)
	$(CARGO) deb --no-build --no-strip --target $(LINUX_AMD64) --output $(DIST)/
	$(CARGO) deb --no-build --no-strip --target $(LINUX_ARM64) --output $(DIST)/
	$(CARGO) deb --no-build --no-strip --variant musl --target $(LINUX_AMD64_MUSL) --output $(DIST)/
	$(CARGO) deb --no-build --no-strip --variant musl --target $(LINUX_ARM64_MUSL) --output $(DIST)/

dist: linux tarball deb ## Build all Linux artifacts (binaries, tarballs, .debs) into dist/
	@echo "── dist/ ──"; ls -1 $(DIST)

dist-clean-deb: ## Remove the dist/ directory
	rm -rf $(DIST)

# ---------- diagnostics ----------

size: ## Show disk usage of target/ subdirectories
	@if [ -d target ]; then \
	    du -sh target target/debug target/release target/doc 2>/dev/null | sort -hr; \
	else \
	    echo "target/ does not exist"; \
	fi

# ---------- meta ----------

bump-check: ## Show current version + release date
	@grep -E '^version' Cargo.toml | head -1
	@grep -E 'RELEASE_DATE' src/version.rs | head -1

ci: fmt-check clippy test ## What CI should run: format check, lint, tests

ci-impersonate: fmt-check clippy test test-impersonate ## CI plus a build+test pass with --features impersonate
