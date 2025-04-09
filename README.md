# rusterize

High performance rasterization tool for Python built in Rust. This
repository stems from the [fasterize](https://github.com/ecohealthalliance/fasterize.git) package built in C++
for R and ports parts of the logics into Python with a Rust backend, in addition to some useful improvements.

**rusterize** is designed to work on *(multi)polygons* and *(multi)linestrings*. Functionally, it takes an input [geopandas](https://geopandas.org/en/stable/) dataframe and returns a [xarray](https://docs.xarray.dev/en/stable/). 

# Installation

Install the current version with pip:

``` shell
pip install rusterize
```

# Contributing

Any contribution is welcome! You can install **rusterize** directly
from this repo using [maturin](https://www.maturin.rs/) as an editable
package. For this to work, you’ll need to have [Rust](https://www.rust-lang.org/tools/install) and
[cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
installed.

``` shell
# Clone repo
git clone https://github.com/<username>/rusterize.git
cd rusterize

# Install the Rust nightly toolchain
rustup toolchain install nightly-2025-01-05

 # Install maturin
pip install maturin

# Install editable version with optmized code
maturin develop --profile dist-release
```

# API

This function has a simple API:

``` python
from rusterize.core import rusterize

# gdf = <import/modify dataframe as needed>

# rusterize
rusterize(gdf,
          res=(30, 30),
          out_shape=(10, 10)
          extent=(0, 300, 0, 300)
          field="field",
          by="by",
          fun="sum",
          background=0) 
```

- `gdf`: geopandas dataframe to rasterize
- `res`: tuple of (xres, yres) for desired resolution
- `out_shape`: tuple of (nrows, ncols) for desired output shape
- `extent`: tuple of (xmin, ymin, xmax, ymax) for desired output extent
- `field`: field to rasterize. Default is None (a value of `1` is rasterized).
- `by`: column to rasterize. Assigns each group to a band in the
  stack. Values are taken from `field`. Default is None (singleband raster)
- `fun`: pixel function to use when multiple values overlap. Default is
  `last`. Available options are `sum`, `first`, `last`, `min`, `max`, `count`, or `any`
- `background`: background value in final raster. Default is None (NaN)

Note that control over the desired extent is not as strict as for resolution and shape. That is,
when resolution, output shape, and extent are specified, priority is given to resolution and shape.
So, extent is not guaranteed, but resolution and shape are. If extent is not given, it is taken
from the polygons and is not modified, unless you specify a resolution value. If you only specify an output
shape, the extent is maintained. This mimics the logics of `gdal_rasterize`.

# Usage

**rusterize** consists of a single function `rusterize()`. The Rust implementation
returns an array that is converted to a xarray on the Python side
for simpliicty.

``` python
from rusterize.core import rusterize
import geopandas as gpd
from shapely import wkt
import matplotlib.pyplot as plt

# Construct geometries
geoms = [
    "POLYGON ((-180 -20, -140 55, 10 0, -140 -60, -180 -20), (-150 -20, -100 -10, -110 20, -150 -20))",
    "POLYGON ((-10 0, 140 60, 160 0, 140 -55, -10 0))",
    "POLYGON ((-125 0, 0 60, 40 5, 15 -45, -125 0))",
    "MULTILINESTRING ((-180 -70, -140 -50), (-140 -50, -100 -70), (-100 -70, -60 -50), (-60 -50, -20 -70), (-20 -70, 20 -50), (20 -50, 60 -70), (60 -70, 100 -50), (100 -50, 140 -70), (140 -70, 180 -50))"
]

# Convert WKT strings to Shapely geometries
geometries = [wkt.loads(geom) for geom in geoms]

# Create a GeoDataFrame
gdf = gpd.GeoDataFrame({'value': range(1, len(geoms) + 1)}, geometry=geometries, crs='EPSG:32619')

# rusterize
output = rusterize(
    gdf,
    res=(1, 1),
    field="value",
    fun="sum"
).squeeze()

# plot it
fig, ax = plt.subplots(figsize=(12, 6))
output.plot.imshow(ax=ax)
plt.show()
```

![](img/plot.png)

# Benchmarks

**rusterize** is fast! Let’s try it on small and large datasets.

``` python
from rusterize.core import rusterize
import geopandas as gpd
import requests
import zipfile
from io import BytesIO

# large dataset (~380 MB)
url = "https://s3.amazonaws.com/hp3-shapefiles/Mammals_Terrestrial.zip"
response = requests.get(url)

# unzip
with zipfile.ZipFile(BytesIO(response.content), 'r') as zip_ref:
    zip_ref.extractall()
    
# read
gdf_large = gpd.read_file("Mammals_Terrestrial/Mammals_Terrestrial.shp")

# small dataset (first 1000 rows)
gdf_small = gdf_large.iloc[:1000, :]

# rusterize at 1/6 degree resolution
def test_large(benchmark):
  benchmark(rusterize, gdf_large, (1/6, 1/6), fun="sum")
   
def test_small(benchmark):
  benchmark(rusterize, gdf_small, (1/6, 1/6), fun="sum")  
```

Then you can run it with [pytest](https://docs.pytest.org/en/stable/) and [pytest-benchmark](https://pytest-benchmark.readthedocs.io/en/stable/):
```
pytest <python file> --benchmark-min-rounds=20 --benchmark-time-unit='s'

--------------------------------------------- benchmark: 1 tests --------------------------------------------
Name (time in s)         Min      Max     Mean  StdDev   Median     IQR  Outliers     OPS  Rounds  Iterations
-------------------------------------------------------------------------------------------------------------
rusterize_large       1.6430   1.9249   1.7442  0.1024   1.6878   0.1974      6;0  0.5733      20           1
rusterize_small       0.0912   0.1194   0.1014  0.0113   0.0953   0.0223      7;0  9.8633      20           1 
-------------------------------------------------------------------------------------------------------------
```

And fasterize:
``` r
library(sf)
library(raster)
library(fasterize)
library(microbenchmark)

large <- st_read("Mammals_Terrestrial/Mammals_Terrestrial.shp", quiet = TRUE)
small <- large[1:1000, ]
fn <- function(v) {
  r <- raster(v, res = 1/6)
  return(fasterize(v, r, fun = "sum"))
}
microbenchmark(
  fasterize_large = f <- fn(large),
  fasterize_small = f <- fn(small),
  times=20L,
  unit='s'
)
```
```
Unit: seconds
            expr       min         lq       mean     median         uq        max neval
 fasterize_large 9.9450280 10.6674467 10.8632224 10.9182963 11.1943478 11.3768210    20
 fasterize_small 0.4906411  0.5140836  0.5581061  0.5320919  0.5603512  0.8750579    20
```
And on an even larger datasets? Here we use a layer from the province of Quebec, Canada representing ~2M polygons of waterbodies, rasterized at 30 meters (20 rounds) with no field value and pixel function `any`. The comparison with `gdal_rasterize` was run with `hyperfine --runs 20 "gdal_rasterize -tr 30 30 -burn 1 <data_in> <data_out>"`.
```
# rusterize
--------------------------------------------- benchmark: 1 tests --------------------------------------------
Name (time in s)         Min      Max     Mean  StdDev   Median     IQR  Outliers     OPS  Rounds  Iterations
-------------------------------------------------------------------------------------------------------------
rusterize             6.7270   7.0098   6.7824  0.0646   6.7686   0.0266      2;2  0.1474      20           1
-------------------------------------------------------------------------------------------------------------

# fasterize
Unit: seconds
      expr      min       lq     mean   median       uq      max neval
 fasterize 157.4734 177.2055 194.3222 194.6455 213.9195 230.6504    20

# gdal_rasterize (CLI) - read from fast drive, write to fast drive
Time (mean ± σ):      5.801 s ±  0.124 s    [User: 4.381 s, System: 1.396 s]
Range (min … max):    5.649 s …  6.023 s    20 runs
```
In terms of (multi)line rasterization speed, here's a benchmark against `gdal_rasterize` using a layer from the province of Quebec, Canada, representing a subset of the road network for a total of ~535K multilinestrings.
```
# rusterize
--------------------------------------------- benchmark: 1 tests --------------------------------------------
Name (time in s)         Min      Max     Mean  StdDev   Median     IQR  Outliers     OPS  Rounds  Iterations
-------------------------------------------------------------------------------------------------------------
test                  4.5272   5.9488   4.7171  0.3236   4.6360  0.1680       2;2  0.2120      20           1
-------------------------------------------------------------------------------------------------------------

# gdal_rasterize (CLI) - read from fast drive, write to fast drive
Time (mean ± σ):      8.719 s ±  0.063 s    [User: 3.782 s, System: 4.917 s]
Range (min … max):    8.658 s …  8.874 s    20 runs
```
# Comparison with other tools

While **rusterize** is fast, there are other fast alternatives out there, including `GDAL`, `rasterio` and `geocube`. However, **rusterize** allows for a seamless, Rust-native processing with similar or lower memory footprint that doesn't require you to leave Python, and returns the geoinformation you need for downstream processing with ample control over resolution, shape, and extent.

The following is a time comparison on a single run on the same larger dataset used earlier.
```
rusterize:    6.7 sec
rasterio:     68  sec (but no spatial information)
fasterize:    157 sec (including raster creation)
geocube:      260 sec (larger memory footprint)
```