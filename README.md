# rusterize

High performance rasterization tool for Python built in Rust. This repository stems from the [fasterize](https://github.com/ecohealthalliance/fasterize.git) package built in C++ for R and ports parts of the logics into Python with a Rust backend, in addition to some useful improvements (see [API](#API)).

**rusterize** is designed to work on _(multi)polygons_ and _(multi)linestrings_, even when they are nested inside complex geometry collections. Functionally, it takes an input [geopandas](https://geopandas.org/en/stable/) dataframe and returns a [xarray](https://docs.xarray.dev/en/stable/), a [numpy](https://numpy.org/), or a sparse array in COOrdinate format.

# Installation

`rusterize` is distributed in two flavors. A `core` library that performs the rasterization and returns a bare `numpy` array, or a `xarray`-flavored version that returns a georeferenced `xarray`. This latter requires `xarray` and `rioxarray` to be installed. This is the recommended flavor.

Install the current version with pip:

```shell
# Core library
pip install rusterize

# With xarray capabilities
pip install 'rusterize[xarray]'
```

# Contributing

Any contribution is welcome! You can install **rusterize** directly from this repo using [maturin](https://www.maturin.rs/) as an editable package. For this to work, you’ll need to have [Rust](https://www.rust-lang.org/tools/install) and [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
installed.

```shell
# Clone repo
git clone https://github.com/<username>/rusterize.git
cd rusterize

# Install the Rust nightly toolchain
rustup toolchain install nightly-2025-07-31

 # Install maturin
pip install maturin

# Install editable version with optmized code
maturin develop --profile dist-release
```

# API

This package has a simple API:

```python
from rusterize import rusterize

# gdf = <geodataframe>

# rusterize
rusterize(
    gdf,
    like=None,
    res=(30, 30),
    out_shape=(10, 10),
    extent=(0, 10, 10, 20),
    field="field",
    by="by",
    burn=None,
    fun="sum",
    background=0,
    encoding="xarray",
    dtype="uint8"
)
```

- `gdf`: geopandas dataframe to rasterize
- `like`: xr.DataArray to use as template for `res`, `out_shape`, and `extent`. Mutually exclusive with these parameters (default: `None`)
- `res`: (xres, yres) for desired resolution (default: `None`)
- `out_shape`: (nrows, ncols) for desired output shape (default: `None`)
- `extent`: (xmin, ymin, xmax, ymax) for desired output extent (default: `None`)
- `field`: column to rasterize. Mutually exclusive with `burn` (default: `None` -> a value of `1` is rasterized)
- `by`: column for grouping. Assign each group to a band in the stack. Values are taken from `field` if specified, else `burn` is rasterized (default: `None` -> singleband raster)
- `burn`: a single value to burn. Mutually exclusive with `field` (default: `None`). If no field is found in `gdf` or if `field` is `None`, then `burn=1`
- `fun`: pixel function to use when multiple values overlap. Available options are `sum`, `first`, `last`, `min`, `max`, `count`, or `any` (default: `last`)
- `background`: background value in final raster (default: `np.nan`). A `None` value corresponds to the default of the specified dtype. An illegal value for a dtype will be replaced with the default of that dtype. For example, a `background=np.nan` for `dtype="uint8"` will become `background=0`, where `0` is the default for `uint8`.
- `encoding`: defines the output format of the rasterization. This is either a dense xarray/numpy representing the burned rasterized geometries, or a sparse array in COOrdinate format good for sparse observations and low memory consumption. Available options are `xarray`, `numpy`, `sparse` (default: `xarray` -> will trigger an error if `xarray` and `rioxarray` are not found).
- `dtype`: dtype of the final raster. Available options are `uint8`, `uint16`, `uint32`, `uint64`, `int8`, `int16`, `int32`, `int64`, `float32`, `float64` (default: `float64`)

Note that control over the desired extent is not as strict as for resolution and shape. That is,
when resolution, output shape, and extent are specified, priority is given to resolution and shape.
So, extent is not guaranteed, but resolution and shape are. If extent is not given, it is taken
from the polygons and is not modified, unless you specify a resolution value. If you only specify an output
shape, the extent is maintained. This mimics the logics of `gdal_rasterize`.

# Encoding

Version 0.5.0 introduced a new `encoding` parameter to control the output format of the rasterization. This means that you can return a xarray/numpy with the burned rasterized geometries, or a new `SparseArray` structure. This `SparseArray` structure stores the band/row/column triplets of where the geometries should be burned onto the final raster, as well as their corresponding values before applying any pixel function. This can be used as an intermediate output to avoid allocating memory before materializing the final raster, or as a final product. `SparseArray` has three convenience functions: `to_xarray()`, `to_numpy()`, and `to_frame()`. The first two return the final xarray/numpy, the last returns a polars dataframe with only the coordinates and values of the rasterized geometries. Note that `SparseArray` avoids allocating memory for the array during rasterization until when it's actually needed (e.g. calling `to_xarray()`). See below for an example.

# Usage

**rusterize** consists of a single function `rusterize()`.

```python
from rusterize import rusterize
import geopandas as gpd
from shapely import wkt
import matplotlib.pyplot as plt

# Construct geometries
geoms = [
    "POLYGON ((-180 -20, -140 55, 10 0, -140 -60, -180 -20), (-150 -20, -100 -10, -110 20, -150 -20))",
    "POLYGON ((-10 0, 140 60, 160 0, 140 -55, -10 0))",
    "POLYGON ((-125 0, 0 60, 40 5, 15 -45, -125 0))",
    "MULTILINESTRING ((-180 -70, -140 -50), (-140 -50, -100 -70), (-100 -70, -60 -50), (-60 -50, -20 -70), (-20 -70, 20 -50), (20 -50, 60 -70), (60 -70, 100 -50), (100 -50, 140 -70), (140 -70, 180 -50))",
    "GEOMETRYCOLLECTION (POINT (50 -40), POLYGON ((75 -40, 75 -30, 100 -30, 100 -40, 75 -40)), LINESTRING (60 -40, 80 0), GEOMETRYCOLLECTION (POLYGON ((100 20, 100 30, 110 30, 110 20, 100 20))))"
]

# Convert WKT strings to Shapely geometries
geometries = [wkt.loads(geom) for geom in geoms]

# Create a GeoDataFrame
gdf = gpd.GeoDataFrame({'value': range(1, len(geoms) + 1)}, geometry=geometries, crs='EPSG:32619')

# rusterize to "xarray" -> return a xarray with the burned geometries and spatial reference (default)
# will raise a ModuleNotFoundError if xarray and rioxarray are not found
output = rusterize(
    gdf,
    res=(1, 1),
    field="value",
    fun="sum",
).squeeze()

# plot it
fig, ax = plt.subplots(figsize=(12, 6))
output.plot.imshow(ax=ax)
plt.show()

# rusterize to "sparse" -> custom structure storing the coordinates and values of the rasterized geometries
output = rusterize(
    gdf,
    res=(1, 1),
    field="value",
    fun="sum",
    encoding="sparse"
)
output
# SparseArray:
# - Shape: (131, 361)
# - Extent: (-180.5, -70.5, 180.5, 60.5)
# - Resolution: (1.0, 1.0)
# - EPSG: 32619
# - Estimated size: 378.33 KB

# materialize into xarray or numpy
array = output.to_xarray()
array = output.to_numpy()

# get only coordinates and values
output.to_frame()
# shape: (29_340, 3)
# ┌─────┬─────┬──────┐
# │ row ┆ col ┆ data │
# │ --- ┆ --- ┆ ---  │
# │ u32 ┆ u32 ┆ f64  │
# ╞═════╪═════╪══════╡
# │ 6   ┆ 40  ┆ 1.0  │
# │ 6   ┆ 41  ┆ 1.0  │
# │ 6   ┆ 42  ┆ 1.0  │
# │ 7   ┆ 39  ┆ 1.0  │
# │ 7   ┆ 40  ┆ 1.0  │
# │ …   ┆ …   ┆ …    │
# │ 64  ┆ 258 ┆ 1.0  │
# │ 63  ┆ 259 ┆ 1.0  │
# │ 62  ┆ 259 ┆ 1.0  │
# │ 61  ┆ 260 ┆ 1.0  │
# │ 60  ┆ 260 ┆ 1.0  │
# └─────┴─────┴──────┘
```

![](img/plot.png)

# Benchmarks

**rusterize** is fast! Let’s try it on small and large datasets.

```python
import zipfile
from io import BytesIO

from pyogrio import read_dataframe
import requests
from rusterize import rusterize

# POLYGON ~468MB
url = "https://ftp.maps.canada.ca/pub/nrcan_rncan/vector/canvec/shp/Hydro/canvec_50K_BC_Hydro_shp.zip"  # ~3.8GB
response = requests.get(url)

with zipfile.ZipFile(BytesIO(response.content), "r") as zip_ref:
    for file_name in zip_ref.namelist():
        if file_name == "canvec_50K_BC_Hydro/waterbody_2":
            zip_ref.extract(file_name)

water_large = read_dataframe("canvec_50K_BC_Hydro/waterbody_2.shp")
water_small = water_large.iloc[:1000, :]


# LINESTRING ~900MB
url = "https://www12.statcan.gc.ca/census-recensement/2011/geo/RNF-FRR/files-fichiers/lrnf000r25p_e.zip"
response = requests.get(url)

with zipfile.ZipFile(BytesIO(response.content), "r") as zip_ref:
    zip_ref.extractall()

roads = read_dataframe("lrnf000r25p_e/lrnf000r25p_e.gpkg")


# BENCHMARK
def test_water_large(benchmark):
    # 1/6 degree resolution
    benchmark(rusterize, water_large, res=(1 / 6, 1 / 6), dtype="uint8")


def test_water_small(benchmark):
    # 1/6 degree resolution
    benchmark(rusterize, water_small, res=(1 / 6, 1 / 6), dtype="uint8")


def test_roads(benchmark):
    # 50 meters, else OOM error on my machine
    benchmark(rusterize, roads, res=(50, 50), dtype="uint8")
```

Then you can run it with [pytest](https://docs.pytest.org/en/stable/) and [pytest-benchmark](https://pytest-benchmark.readthedocs.io/en/stable/):

```
pytest <python file> --benchmark-min-rounds=10 --benchmark-time-unit='s'

--------------------------------------------- benchmark: 1 tests ---------------------------------------------------
Name (time in s)         Min      Max     Mean    StdDev   Median      IQR  Outliers        OPS  Rounds  Iterations
--------------------------------------------------------------------------------------------------------------------
test_water_small      0.0037    0.0048   0.0039   0.0002   0.0039   0.0002      35;5  255.1552     202            1
test_water_large      1.0108    1.1280   1.0355   0.0437   1.0165   0.0077       2;2    0.9657      10            1
test_roads            3.5131    3.8548   3.5942   0.1115   3.5450   0.0622       2;2    0.2782      10            1
--------------------------------------------------------------------------------------------------------------------
```

And fasterize (note that it doesn't support custom `dtype` so the returning raster is `float64`):

```r
library(sf)
library(raster)
library(fasterize)
library(microbenchmark)

# polygon data only
large <- st_read("canvec_50K_BC_Hydro/waterbody_2.shp", quiet = TRUE)
small <- large[1:1000, ]

fn <- function(v) {
  r <- raster(v, res = 1 / 6)
  fasterize(v, r)
}

microbenchmark(
  fasterize_large = f <- fn(large),
  fasterize_small = f <- fn(small),
  times = 10L,
  unit = "s"
)
```

```
Unit: seconds
            expr         min          lq       mean      median         uq        max neval
 fasterize_small  0.05764281  0.06274373  0.1286875  0.06520358  0.1128432  0.6000182    10
 fasterize_large 36.91321005 37.71877265 41.0140303 40.81343803 43.9201820 46.5596799    10
```

The comparison with `gdal_rasterize` was run with `hyperfine --runs 10 "gdal_rasterize -tr <xres> <yres> -burn 1 -ot Byte <data_in> <data_out>"`. Note that GDAL needs to read the geometries first, hence the great discrepancy with `rusterize` for linestring rasterization. Including the read time using [pyogrio](https://github.com/geopandas/pyogrio) adds approximately 19 seconds.

```
# POLYGONS: gdal_rasterize (CLI) - read from fast drive, write to fast drive
Time (mean ± σ):      2.306 s ±  0.016 s    [User: 1.978 s, System: 0.327 s]
Range (min … max):    2.278 s …  2.333 s    10 runs

# LINESTRINGS: gdal_rasterize (CLI) - read from fast drive, write to fast drive
Time (mean ± σ):     10.970 s ±  0.779 s    [User: 4.996 s, System: 5.951 s]
Range (min … max):   10.633 s … 13.162 s    10 runs
```

# Comparison with other tools

While **rusterize** is fast, there are other fast alternatives out there, including `rasterio` and `geocube`. However, **rusterize** allows for a seamless, Rust-native processing with similar or lower memory footprint that doesn't require you to install GDAL and returns the geoinformation you need for downstream processing with ample control over resolution, shape, extent, and data type.

The following is a time comparison on 10 runs (median) on the same large water bodies dataset used earlier.

```
rusterize: 1.0 sec
rasterio:  14.8 sec
geocube:   129.2 sec
```

# Integrations

Happy to share that **rusterize** has been integrated into the following libraries:

- [rasterix](https://github.com/xarray-contrib/rasterix)
