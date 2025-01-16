#!/bin/bash

# build docker wheel
docker run --rm -v $(pwd):/io quay.io/pypa/manylinux_2_28_x86_64 maturin build --profile dist-release --release

# publish on TestPypu
maturin upload -r https://test.pypi.org/legacy/ ~/Downloads/z_base_32-0.1.0-py3-none-manylinux_2_5_x86_64.manylinux1_x86_64.whl
