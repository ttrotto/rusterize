## rusterize

**rusterize** is an extremely fast rasterization engine built in 🦀 Rust and ported to R.

It is designed to work on _all_ [`sf`](https://r-spatial.github.io/sf/)-supported geometries, even when they are nested inside complex geometry collections.
Currently, it supports only `sf` dataframes as input.

It returns a [`terra`](https://rspatial.github.io/terra/) object, or a custom sparse array in COOrdinate format.

Visit the full documentation [here](ttrotto.github.io/rusterize).
