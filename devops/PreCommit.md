# Introduction

_[Return](README.md)_

This is not a full pre-commit documentation, just basic notes of the ivynet setup.
For more information on the tool, check the [official website](https://pre-commit.com/).

# Usage

Precommit is run with every git commit by [this workflow](github/workflows/pre-commit.yml).

It can be also run manually with:
```
pre-commit run --show-diff-on-failure --all-files
```


# Documentation

Precommit definitions are store in the [.pre-commit-config.yaml](../.pre-commit-config.yaml) file.

## Used packages

- basic hooks (https://github.com/pre-commit/pre-commit-hooks)
- rust hooks (https://github.com/ivy-net/pre-commit-rust)

The rust one is [our fork](https://github.com/ivy-net/pre-commit-rust) of an old community repo.
It includes +nightly for fmt and potential to add other commands.


## Setup

First install and then update to the latest version of the plugins.
```
pre-commit install
pre-commit autoupdate
```
## Installation notes

### Rust tooling

* Install rust tooling (e.g. with rustup https://www.rust-lang.org/tools/install)
* Add extra pacake: [cargo-machete](https://github.com/bnjbvr/cargo-machete)

### Pre Commit (on MacOS)

* Install pre-commit
```
brew install pre-commit
```

_[Return](README.md)_
