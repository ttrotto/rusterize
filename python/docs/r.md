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

Check out the Python [benchmarks](python.md#benchmarks) for a comparison with GDAL.

Benchmark against `fasterize` ([benchmark_rusterize.r](https://github.com/ttrotto/rusterize/blob/c3f60249e213753e45e721fb25ebe6519050a884/R/rusterize/benchmarks/benchmark.r)) with dtype "double".
`fasterize` requires a template `raster::RasterLayer` to be built before rasterization, so these benchmarks include the time to generate this template.

```r
Unit: seconds
                expr         min          lq        mean      median          uq         max neval
 rusterize_large_f64  1.51818793  1.66960548  1.78801887  1.71947791  1.84290258  2.26121972    10
 rusterize_small_f64  0.01544157  0.01549703  0.01670145  0.01616975  0.01824702  0.01928829    10
 fasterize_large_f64 36.75667095 37.33715239 40.80729938 40.78636142 44.06979276 45.36614083    10
 fasterize_small_f64  0.06376405  0.06484535  0.08900962  0.07315597  0.07809348  0.25425766    10
```

And without the template creation time (**rusterize** still needs to create the array internally):

```r
Unit: seconds
                expr      min       lq     mean   median       uq      max neval
 rusterize_large_f64 1.583420 1.723896 1.906175 1.913970 2.006997 2.377668    10
 rusterize_small_f64 0.013093 0.013721 0.020528 0.014113 0.014475 0.078459    10
 fasterize_large_f64 2.149883 2.684711 3.122519 3.036046 3.461804 4.139388    10
 fasterize_large_f64 0.004977 0.005224 0.005533 0.005357 0.005896 0.006376    10
```
