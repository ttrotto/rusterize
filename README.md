# rusterize
High performance rasterization tool for python built in Rust. This repository is heavily based on the **fasterize** package build in C++ for R. This version ports it to Python with a Rust backend.

Functionally, it works on [geopandas](https://geopandas.org/en/stable/) dataframes and utilizes [polars](https://pola.rs/) on the Rust end. It tighly mirrors the processing routine of fasterize, so it works only on (multi)polygon geometries.

# Installation
Install the current version from pip:

```shell
pip install rusterize
```

You can also build the wheel directly from this repo using [maturin](https://www.maturin.rs/) as an editable package.

```shell
maturin develop --release --profile dist-release
```

# Usage
It consists of a single function `rusterize()` that takes as input a geopandas dataframe and returns a [xarray](https://docs.xarray.dev/en/stable/). The Rust implementation returns an array that is then converted to an xarray on the Python side for simpliicty.

```python
from rusterize.core import rusterize
import pandas as pd
import geopandas ad gpd
import matplotlib.pyplot as plt

# example taken from geopandas website
df = pd.DataFrame(
    {
        "City": ["Buenos Aires", "Brasilia", "Santiago", "Bogota", "Caracas"],
        "Country": ["Argentina", "Brazil", "Chile", "Colombia", "Venezuela"],
        "Latitude": [-34.58, -15.78, -33.45, 4.60, 10.48],
        "Longitude": [-58.66, -47.91, -70.66, -74.08, -66.86],
    }
)

gdf = geopandas.GeoDataFrame(
    df, geometry=geopandas.points_from_xy(df.Longitude, df.Latitude), crs="EPSG:4326"
).buffer(10)

output = rusterize(
    gdf,
    res=(10, 10),
    field="Country",
    fun="sum",
    background=0
)

output.plot.imshow()
plt.show()
```

# Documentation
rusterize takes the same arguments as fasterize and implements a parallel execution for calls where `by` is specified. A thread count of 4 is considered good for most applications. [Here]() the documentation for the small API reference.

# Benchmark
fasterize is fast and so is rusterize!



