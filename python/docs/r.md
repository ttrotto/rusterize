# rusterize in R

**rusterize** is an extremely fast rasterization engine built in 🦀 Rust and ported to R.

It is designed to work on _all_ [`sf`](https://r-spatial.github.io/sf/)-supported geometries, even when they are nested inside complex geometry collections.
Currently, it supports only `sf` dataframes as input.

It returns a [`terra`](https://rspatial.github.io/terra/) object, or a custom sparse array in COOrdinate format.

## Installation

To avoid messing with the R base environment, you can use [`rv`](https://a2-ai.github.io/rv-docs/) to create a new environment to install **rusterize**.

### Using rv

First, configure the appropriate repository:

```bash
rv configure repository add "rusterize" --url "https://ttrotto.r-universe.dev"
```

Then add it and sync:

```bash
rv add rusterize --repository "rusterize"
```

Other ways to install **rusterize**:

- `install.packages("rusterize", repos = "https://ttrotto.r-universe.dev")`
- `pak::repo_add("https://ttrotto.r-universe.dev")` -> `pak::pkg_install("rusterize")`
- `renv::install("rusterize", repos = "https://ttrotto.r-universe.dev")`.

### Usage

Visit the full [API reference](r_api.md).

Note that **rusterize** always returns a 3-dimensional array as a convention, even if there is only one band.

```r
library(rusterize)
library(sf)
library(terra)

# construct geometries
geoms <- c(
  "POLYGON ((-180 -20, -140 55, 10 0, -140 -60, -180 -20), (-150 -20, -100 -10, -110 20, -150 -20))",
  "POLYGON ((-10 0, 140 60, 160 0, 140 -55, -10 0))",
  "POLYGON ((-125 0, 0 60, 40 5, 15 -45, -125 0))",
  "MULTILINESTRING ((-180 -70, -140 -50), (-140 -50, -100 -70), (-100 -70, -60 -50), (-60 -50, -20 -70), (-20 -70, 20 -50), (20 -50, 60 -70), (60 -70, 100 -50), (100 -50, 140 -70), (140 -70, 180 -50))",
  "GEOMETRYCOLLECTION (POINT (50 -40), POLYGON ((75 -40, 75 -30, 100 -30, 100 -40, 75 -40)), LINESTRING (60 -40, 80 0), GEOMETRYCOLLECTION (POLYGON ((100 20, 100 30, 110 30, 110 20, 100 20))))"
)

# create an sf dataframe with geometries parsed from WKT
gdf <- st_sf(value = seq_along(geoms), geometry = st_as_sfc(geoms, crs = 32619))

output <- rusterize(
  gdf,
  resolution = c(1, 1),
  field = "value",
  fun = "sum"
)

# plot it
plot(output)
```

You could also create a multiband output by specifying the `by` parameter.

```r
gdf$by <- c("a", "a", "b", "b", "c")

output <- rusterize(
  gdf,
  resolution = c(1, 1),
  field = "value",
  by = "by",
  fun = "sum"
)
```

Alternatively, you can pass raw values to burn on the final raster, one per geometry.

```r
output <- rusterize(
  st_geometry(gdf),
  resolution = c(1, 1),
  fun = "sum",
  burn = seq_along(geoms)
)
```

Finally, you can also create a sparse array in COOrdinate format, that is an object storing the band/row/col value triplets of all pixels that will be materialized in a final raster.

```r
sparse <- rusterize(
  gdf,
  resolution = c(1, 1),
  field = "value",
  fun = "sum",
  encoding = "sparse"
)

# inspect its spatial information, then materialize into a terra SpatRaster on demand
sparse$extent()      # c(xmin, ymin, xmax, ymax)
sparse$resolution()  # c(xres, yres)
sparse$epsg()        # 32619

# materialize into a terra::SpatRaster
sparse$to_raster()
```

## Benchmarks

Check out the Python [benchmarks](python.md#benchmarks) for a comparison with GDAL, `rasterio`, and `geocube`.

Benchmark against `fasterize` ([benchmark_rusterize.r](https://github.com/ttrotto/rusterize/blob/main/R/rusterize/benchmarks/benchmark_rusterize.r)) on the same water bodies
dataset and resolution as the Python benchmarks (1/600°, ~185 × 105 m), burning the `perm` field as dtype "double". `small` is the first 1,000 lakes, `large` is all 458,295.
`fasterize` requires a template `raster::RasterLayer` to be built before rasterization, so these benchmarks include the time to generate this template:

| expr                |     min |      lq |    mean |  median |      uq |     max | neval |
| ------------------- | ------: | ------: | ------: | ------: | ------: | ------: | ----: |
| rusterize_large_f64 |  3.2532 |  3.4713 |  4.2505 |  3.8292 |  5.4470 |  6.1742 |    10 |
| rusterize_small_f64 |  0.2857 |  0.3841 |  0.7996 |  0.4817 |  0.8398 |  2.3760 |    10 |
| fasterize_large_f64 | 36.9160 | 38.2534 | 40.9715 | 40.2552 | 44.3356 | 46.5481 |    10 |
| fasterize_small_f64 |  0.1786 |  0.1816 |  0.2567 |  0.2031 |  0.3116 |  0.5180 |    10 |

And without the template creation time (**rusterize** still needs to create the raster internally):

| expr                |    min |     lq |   mean | median |     uq |    max | neval |
| ------------------- | -----: | -----: | -----: | -----: | -----: | -----: | ----: |
| rusterize_large_f64 | 3.2988 | 3.4865 | 3.6887 | 3.6269 | 3.8646 | 4.2427 |    10 |
| rusterize_small_f64 | 0.3431 | 0.3463 | 0.6871 | 0.4465 | 1.0102 | 1.4237 |    10 |
| fasterize_large_f64 | 2.4164 | 2.5119 | 2.9234 | 2.8285 | 2.8804 | 3.9946 |    10 |
| fasterize_small_f64 | 0.1219 | 0.1231 | 0.2315 | 0.1464 | 0.2244 | 0.8348 |    10 |

Including the template, **rusterize** is about **10×** faster than `fasterize` on the large dataset (3.8 s against 40.3 s, median). With a template already
built, `fasterize` is faster: most of the time of **rusterize** in R goes into
building the output `terra::SpatRaster`, while the rasterization itself is fast (see the Python benchmarks).
