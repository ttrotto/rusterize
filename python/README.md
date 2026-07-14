## rusterize

**rusterize** is an extremely fast rasterization engine built in 🦀 Rust and ported to 🐍 Python.

It is designed to work on _all_ shapely geometries, even when they are nested inside complex geometry collections. Functionally, it supports four input types:

- [geopandas](https://geopandas.org/en/stable/) GeoDataFrame and GeoSeries
- [polars-st](https://oreilles.github.io/polars-st/) GeoDataFrame
- Python list of geometries in shapely.Geometry, WKB, or WKT format
- Numpy array of geometries in shapely.Geometry, WKB, or WKT format

It returns a [xarray](https://docs.xarray.dev/en/stable/), a [numpy](https://numpy.org/), or a custom sparse array in COOrdinate format.

Visit the full documentation [here](https://ttrotto.github.io/rusterize/python/).
