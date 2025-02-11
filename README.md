# rusterize

High performance rasterization tool for Python built in Rust. This
repository is heavily based on the [fasterize](https://github.com/ecohealthalliance/fasterize.git) package built in C++
for R. This version ports it to Python with a Rust backend, with some useful improvements.

Functionally, it takes an input [geopandas](https://geopandas.org/en/stable/)
dataframes and returns a [xarray](https://docs.xarray.dev/en/stable/). It
tighly mirrors the processing routine of fasterize, so it works only on
(multi)polygon geometries at the moment.

# Installation

Install the current version with pip:

``` {shell}
pip install rusterize
```

# Contributing

Any contribution is welcome! You can install **rusterize** directly
from this repo using [maturin](https://www.maturin.rs/) as an editable
package. For this to work, you’ll need to have [Rust](https://www.rust-lang.org/tools/install) and
[cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
installed.

``` {shell}
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

``` {shell}
from rusterize.core import rusterize

# gdf = <import datasets as needed>

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
  stack. Values are taken from `field`. Default is None
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

# example from fasterize
polygons = [
    "POLYGON ((-180 -20, -140 55, 10 0, -140 -60, -180 -20), (-150 -20, -100 -10, -110 20, -150 -20))",
    "POLYGON ((-10 0, 140 60, 160 0, 140 -55, -10 0))",
    "POLYGON ((-125 0, 0 60, 40 5, 15 -45, -125 0))"
]

# Convert WKT strings to Shapely geometries
geometries = [wkt.loads(polygon) for polygon in polygons]

# Create a GeoDataFrame
gdf = gpd.GeoDataFrame({'value': range(1, len(polygons) + 1)}, geometry=geometries, crs='EPSG:32619')

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

Then you can run it with [pytest](https://docs.pytest.org/en/stable/)
and
[pytest-benchmark](https://pytest-benchmark.readthedocs.io/en/stable/):

``` {shell}
pytest <python file> --benchmark-min-rounds=20 --benchmark-time-unit='s'

--------------------------------------------- benchmark: 1 tests --------------------------------------------
Name (time in s)         Min      Max     Mean  StdDev   Median     IQR  Outliers     OPS  Rounds  Iterations
-------------------------------------------------------------------------------------------------------------
test_large           10.5870  11.2302  10.8633  0.1508  10.8417  0.1594       4;1  0.0921      20           1
test_small            0.5083   0.6416   0.5265  0.0393   0.5120  0.0108       2;2  1.8995      20           1
-------------------------------------------------------------------------------------------------------------
```

And fasterize:

``` {r}
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

``` {shell}
Unit: seconds
      expr             min        lq      mean    median        uq       max  neval
 fasterize_large  9.565781  9.815375  10.02838  9.984965  10.18532  10.66656     20
 fasterize_small  0.469389  0.500616  0.571851  0.558818  0.613419  0.795159     20
```

And on even
[larger](https://open.canada.ca/data/en/dataset/fbf12500-bffe-4209-a1ae-fba86f154ebf/resource/cc90d77c-fba3-4f84-b30a-e684cfe0649a)
datasets? This is a benchmark with 350K+ geometries rasterized at 30
meters (20 rounds) with no field value and pixel function `sum`.

``` {shell}
# rusterize
--------------------------------------------- benchmark: 1 tests --------------------------------------------
Name (time in s)         Min      Max     Mean  StdDev   Median     IQR  Outliers     OPS  Rounds  Iterations
-------------------------------------------------------------------------------------------------------------
test_sbw             46.5711  49.0212  48.4340  0.5504  48.5812  0.5054       3;1  0.0206      20           1
-------------------------------------------------------------------------------------------------------------

# fasterize
Unit: seconds
      expr      min       lq     mean   median       uq      max neval
 fasterize 62.12409 72.13832 74.53424 75.12375 77.72899 84.77415    20
```

# Comparison with other tools

While `rusterize` is fast, there are other very fast solutions out there, including:
- `GDAL`
- `rasterio`
- `geocube`

However, `rusterize` allows for a seamless, Rust-native processing with similar or lower memory footprint that doesn't require you to leave Python, and returns the geoinformation you need for downstream processing.

The following is a time comparison run on a dataset with 340K+ geometries, rasterized at 2m resolution.
```
rusterize:   24 sec
fasterize:   47 sec
GDAL (cli):  40 sec (read from fast drive, write to fast drive)
rasterio:    20 sec (but no spatial information)
geocube:     42 sec (larger memory footprint)
```