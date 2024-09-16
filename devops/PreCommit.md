# Introduction

This is not full documentation, just basic notes from my initial run.

# Usage

To run:
```
pre-commit run --show-diff-on-failure --all-files
```
## Setup

First install and then update to the latest version of the plugins.
```
pre-commit install
pre-commit autoupdate
```

# Documentation

https://pre-commit.com/

# Used packages

- basic hooks (https://github.com/pre-commit/pre-commit-hooks)
- rust hooks (https://github.com/ivy-net/pre-commit-rust)

The rust one is our fork (based on fork).
It includes +nightly for fmt and potential to add other commands.

# Installation notes

## Rust tooling

* Install rust tooling (e.g. with rustup https://www.rust-lang.org/tools/install)

### Extra packages

- cargo-machete

## Pre Commit (on MacOS)

* Install pre-commit
```
brew install pre-commit
```
