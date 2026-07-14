# Contributing

Any contribution is welcome! You can install **rusterize** directly from this repo. For this to work, you’ll need to have [Rust](https://www.rust-lang.org/tools/install) and
[cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.

```bash
# clone repo (or fork)
git clone https://github.com/ttrotto/rusterize.git
cd rusterize

# install Rust nightly toolchain
rustup toolchain install nightly-2026-04-01
```

Test the new contribution depending on the port you're working with.

## Rust

Test the codebase and the documentation.

`cargo test -p rusterize-rs` and `cargo test -p rusterize-rs --doc`

## Python

We'll use [maturin](https://www.maturin.rs/) to develop **rusterize** as an editable package in Python. It is also recommended to start with a fresh
environment for testing (e.g. using `[uv](https://docs.astral.sh/uv/)`):

```bash
# install maturin
uv pip install maturin

# install editable version with unoptimized code
maturin develop --uv

# test the new contribution
pytest
```

This will build it with an unoptimized profile. Pass `--profile dist-release` to maturin to build the release distribution.

## R

The R port of **rusterize** is built with [`savvy`](https://yutannihilation.github.io/savvy/guide/), so you'll need to have it installed, as well as `devtools`.
It's good practice to start with a fresh environment for testing (e.g. via [`rv`](https://a2-ai.github.io/rv-docs/)), then inside it, run:

```r
# recompile Rust <-> R wrappers and update documentation
savvy::savvy_update()
devtools::document()

# test the new contribution
testthat::test_local()
```

Alternatively, run [savvy-update.sh](https://github.com/ttrotto/rusterize/blob/c3f60249e213753e45e721fb25ebe6519050a884/R/rusterize/tools/savvy-update.sh)
in a bash shell to update and rebuild the docs. This will build it with an unoptimized profile. To build the release distribution, run `SAVVY_PROFILE=dist-release R CMD INSTALL .`
