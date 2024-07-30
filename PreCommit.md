# Introduction

## Quick notes on pre-commit setup

This is not full documentaiton, just basic notes from my intial run.

## Used packages

- basic hooks (https://github.com/pre-commit/pre-commit-hooks)
- rust (https://github.com/doublify/pre-commit-rust)

The rust one seems to be old, but I could not find better.


# Installation on MacOS

```
brew install pre-commit
brew install rustfmt
```

# Setup

First install and then update to the latest version of the plugins.

```
pre-commit install
pre-commit autoupdate
```
