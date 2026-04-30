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

.DEFAULT_GOAL := help

.PHONY: help build release all check test test-quiet fmt fmt-check clippy lint \
        doc run install uninstall clean clean-all distclean size pdf \
        flags examples bump-check ci

help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make \033[36m<target>\033[0m\n\nTargets:\n"} \
	     /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# ---------- build ----------

build: ## Debug build
	$(CARGO) build

release: ## Release build (optimized)
	$(CARGO) build --release

all: build release ## Build both debug and release (per project convention)

check: ## Fast type-check without producing a binary
	$(CARGO) check --all-targets

# ---------- test / lint ----------

test: ## Run all tests
	$(CARGO) test

test-quiet: ## Run tests with minimal output
	$(CARGO) test --quiet

fmt: ## Format the codebase
	$(CARGO) fmt --all

fmt-check: ## Verify formatting without modifying files
	$(CARGO) fmt --all -- --check

clippy: ## Run clippy with warnings denied
	$(CARGO) clippy --all-targets -- -D warnings

lint: fmt-check clippy ## fmt-check + clippy

# ---------- run / install ----------

run: ## Run the debug binary (use ARGS="..." to pass arguments)
	$(CARGO) run -- $(ARGS)

install: release ## Install the release binary into ~/.cargo/bin
	$(CARGO) install --path . --force

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

clean-all: clean ## clean + remove generated rustdoc and stray artefacts
	rm -rf target/doc dump.rdb

distclean: clean-all ## clean-all + drop Cargo.lock (rarely needed)
	rm -f Cargo.lock

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
