help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}' | sort
.PHONY: help

build: ## Build the project in debug mode
	cargo build
.PHONY: build

release: ## Build the project in release mode
	cargo build --release
.PHONY: release

test: ## Run all tests
	${MAKE} build
	cargo test
.PHONY: test

check: ## Quick check without building
	cargo check
.PHONY: check

lint: ## Lint project
	${MAKE} clippy
	${MAKE} fmt
.PHONY: lint

clippy: ## Run clippy
	cargo clippy -- -D warnings
.PHONY: clippy

fmt: ## Format code with rustfmt
	cargo fmt
.PHONY: fmt

fmt-check: ## Check formatting without modifying
	cargo fmt -- --check
.PHONY: fmt-check

run: ## Run the project
	cargo run
.PHONY: run

run-release: ## Run in release mode
	cargo run --release
.PHONY: run-release

clean: ## Clean build artifacts
	cargo clean
.PHONY: clean

doc: ## Generate documentation
	cargo doc --no-deps --open
.PHONY: doc

bench: ## Run benchmarks
	cargo bench
.PHONY: bench

watch: ## Watch for changes and rebuild (requires cargo-watch)
	cargo watch -x build
.PHONY: watch

watch-test: ## Watch and run tests
	cargo watch -x test
.PHONY: watch-test

audit: ## Audit dependencies for security issues (requires cargo-audit)
	cargo audit
.PHONY: audit

update: ## Update dependencies
	cargo update
.PHONY: update

install: ## Install the binary
	cargo install --path .
.PHONY: install

all: ## Run all checks (format, lint, test, build)
	$(MAKE) fmt
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) build
.PHONY: all

ci: ## CI target - all checks without interactive elements
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) build
.PHONY: ci

coverage: ## Generate code coverage (requires tarpaulin)
	cargo tarpaulin --out Html
.PHONY: coverage

expand: ## Expand macros (useful for debugging)
	cargo expand
.PHONY: expand

init:
	${MAKE} setup-hooks
.PHONY: init

setup-hooks: ## Install pre-commit hooks using prek
	@command -v prek >/dev/null 2>&1 || { echo >&2 "prek is not installed. Install it with: cargo install --root $PWD prek"; exit 1; }
	prek install
.PHONY: setup-hooks
