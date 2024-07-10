# Introduction

To run:
```
pre-commit run --show-diff-on-failure --all-files
```

# Setup

## Quick notes on pre-commit setup

This is not full documentation, just basic notes from my initial run.

## Used packages

- basic hooks (https://github.com/pre-commit/pre-commit-hooks)
- rust hooks (https://github.com/ivy-net/pre-commit-rust)

The rust one is our fork (based on fork).
It includes +nightly for fmt and potential to add other commands.

# Installation

## Rust tooling

* Install rust tooling (e.g. with rustup https://www.rust-lang.org/tools/install)

## Pre Commit (on MacOS)

* Install pre-commit
```
brew install pre-commit
```

# Setup

First install and then update to the latest version of the plugins.
```
pre-commit install
pre-commit autoupdate
```
