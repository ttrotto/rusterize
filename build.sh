#!/bin/bash

# ideally run cargo clippy before maturin

# build with dist-release profile
maturin develop --profile dist-release
