.DEFAULT_GOAL := help

CARGO ?= cargo
DX ?= dx
ARGS ?=

.PHONY: help run dev demo gallery build test check fmt fmt-check lint verify bundle clean

define HELP_TEXT

InputForge development commands
Usage: make <command>

  Launch
    run        Launch the app with Dioxus
    dev        Launch the app with hot reload
    demo       Launch the seeded GUI demo with hot reload
    gallery    Launch the component gallery with hot reload

  Build and test
    build      Build the workspace
    test       Run workspace tests
    check      Check all workspace targets

  Code quality
    fmt        Format the workspace
    fmt-check  Check workspace formatting
    lint       Run Clippy with warnings treated as errors
    verify     Run fmt-check, lint, and test in sequence

  Packaging and maintenance
    bundle     Build a local Windows NSIS release installer
    clean      Remove Cargo build artifacts
    help       Show this help

Overrides: CARGO=cargo DX=dx ARGS="extra arguments"
Linux and Windows support app startup. NSIS installer packaging requires Windows.

endef

help:
	$(info $(HELP_TEXT))
	@exit 0

run:
	"$(DX)" run -p inputforge-app $(ARGS)

dev:
	"$(DX)" serve -p inputforge-app --platform desktop $(ARGS)

demo:
	"$(DX)" serve -p inputforge-gui-dx --example bridge_demo --platform desktop $(ARGS)

gallery:
	"$(DX)" serve -p inputforge-gui-dx --example component_gallery --platform desktop $(ARGS)

build:
	"$(CARGO)" build --workspace --locked $(ARGS)

test:
	"$(CARGO)" test --workspace --locked $(ARGS)

check:
	"$(CARGO)" check --workspace --all-targets --locked $(ARGS)

fmt:
	"$(CARGO)" fmt --all

fmt-check:
	"$(CARGO)" fmt --all -- --check

lint:
	"$(CARGO)" clippy --workspace --all-targets --locked -- -D warnings

# Separate recursive calls preserve ordering and stop on failure, even with -j.
verify:
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test

bundle:
ifeq ($(OS),Windows_NT)
	"$(DX)" bundle --package inputforge-app --bin inputforge --platform windows --release --package-types nsis --locked $(ARGS)
else
	@echo The bundle target requires Windows to build the NSIS installer.
	@exit 1
endif

clean:
	"$(CARGO)" clean
