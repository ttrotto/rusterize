#!/bin/bash

# activate venv
source .venv/bin/activate

# build with dist-release profile
maturin develop --profile dist-release
