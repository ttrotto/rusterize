# R API reference

## rusterize

```r
rusterize(
  data,
  like = NULL,
  resolution = NULL,
  out_shape = NULL,
  extent = NULL,
  field = NULL,
  by = NULL,
  burn = NULL,
  fun = "last",
  background = NA,
  encoding = "dense",
  all_touched = FALSE,
  tap = FALSE,
  dtype = "double"
)
```

Extremely fast rasterization engine built in Rust for R.

The output grid is defined either from a `like` template, or from a combination of `resolution`,
`out_shape`, and `extent`. `like` is mutually exclusive with the latter three. When no `extent` is
given, it is derived from the bounding box of `data`.

### Parameters

**`data`**
: An `sf` or `sfc` object of geometries. All `sf`-supported geometries are accepted, including nested
geometry collections.

**`like`**
: A `terra` template used as a spatial blueprint (resolution, dimension, extent). Mutually exclusive
with `resolution`, `out_shape`, and `extent`.

**`resolution`**
: Pixel resolution as `c(xres, yres)` (or a list). Mutually exclusive with `out_shape`.

**`out_shape`**
: Output raster dimensions as `c(nrows, ncols)` (or a list). Mutually exclusive with `resolution`.

**`extent`**
: Spatial bounding box as `c(xmin, ymin, xmax, ymax)` (or a list). Requires `resolution` or
`out_shape` to also be set.

**`field`**
: Column name to use for pixel values. Mutually exclusive with `burn`.

**`by`**
: Column used for grouping. Each group is rasterized into a distinct band in the output.

**`burn`**
: A static value or a vector of values to apply to each geometry. If a vector, it must match the
length of the geometry data. Mutually exclusive with `field`. If a vector, its dtype should match the
output dtype, else it will be internally casted.

**`fun`**
: Pixel function to use when burning geometries. Available options: `"sum"`, `"first"`, `"last"`,
`"min"`, `"max"`, `"count"`, or `"any"`. Defaults to `"last"`.

**`background`**
: Value assigned to pixels not covered by any geometry. Defaults to `NA`.

**`encoding`**
: The format of the returned object: `"dense"` (default) or `"sparse"`. See
[`SparseArray`](#sparsearray).

**`all_touched`**
: If `TRUE`, every pixel touched by a geometry is burned. Defaults to `FALSE`.

**`tap`**
: Target Align Pixel: aligns the extent to the pixel resolution. Defaults to `FALSE`.

**`dtype`**
: Output data type: `"integer"` or `"double"`. Defaults to `"double"`.

### Returns

A `terra::SpatRaster` when `encoding = "dense"`, or a [`SparseArray`](#sparsearray) when
`encoding = "sparse"`. The raster is always 3-dimensional (band, row, col) by convention, even with a
single band.

## SparseArray

Returned when `encoding = "sparse"`. This internal structure holds the triplets of (band, row, col)
values for each lazily burned pixel, thus avoiding to materialize a full array in memory. This is
advantageous when you have large empty portions or want to save memory until you actually need it.

A `SparseArray` exposes its spatial information and one converter:

- `epsg()` &rarr; `integer` EPSG code
- `extent()` &rarr; `c(xmin, ymin, xmax, ymax)`
- `resolution()` &rarr; `c(xres, yres)`
- `names()` &rarr; band names (`character`)
- `to_raster()` &rarr; `terra::SpatRaster`

```r
sparse <- rusterize(gdf, resolution = c(1, 1), field = "value", fun = "sum", encoding = "sparse")

sparse$extent()      # c(xmin, ymin, xmax, ymax)
sparse$resolution()  # c(xres, yres)
sparse$epsg()        # 32619

sparse$to_raster()   # materialize into a terra::SpatRaster
```
