#' Extremely fast rasterization engine built in Rust for R
#'
#' The output grid is defined either from a `like` template, or from a combination of `resolution`,
#' `out_shape`, and `extent`. When no `extent` is given, it is derived from the bounding box of `data`.
#'
#' @param data An `sf` or `sfc` object of geometries. All `sf`-supported geometries are accepted, including nested
#' geometry collections.
#' @param like A `terra::SpatRaster` template used as a spatial blueprint (resolution, dimension, extent).
#' Mutually exclusive with `resolution`, `out_shape`, and `extent`.
#' @param resolution Pixel resolution as `c(xres, yres)` (or a list). Mutually exclusive with `out_shape`.
#' @param out_shape Output raster dimensions as `c(nrows, ncols)` (or a list). Mutually exclusive with `resolution`.
#' @param extent Spatial bounding box as `c(xmin, ymin, xmax, ymax)` (or a list). Requires `resolution` or
#' `out_shape` to also be set.
#' @param field Column name to use for pixel values. Mutually exclusive with `burn`.
#' @param by Column used for grouping. Each group is rasterized into a distinct band in the output. `NA` values
#' raise an error; drop them first.
#' @param burn A static value or a numeric vector of values, one per geometry. If a vector, it must match the
#' length of the geometry data. Mutually exclusive with `field`. Values are converted to the output `dtype`;
#' with `dtype = "integer"`, a value that is not a whole number or is out of range raises an error.
#' @param fun Pixel function to use when burning geometries. Available options: `"sum"`, `"first"`, `"last"`,
#' `"min"`, `"max"`, `"count"`, or `"any"`. `"sum"` and `"count"` add each pixel at most once per geometry
#' (like GDAL's `MERGE_ALG=ADD`), even where a line crosses itself. With `dtype = "integer"` they wrap around on
#' overflow past 2^31 - 1, so use `"double"` for very large totals.
#' @param background Value assigned to pixels not covered by any geometry.
#' @param encoding The format of the returned object: `"dense"` or `"sparse"`.
#' @param all_touched If `TRUE`, every pixel touched by a geometry is burned.
#' @param tap Target Aligned Pixels: aligns the extent to the pixel resolution.
#' @param dtype Output data type: `"integer"` or `"double"`.
#'
#' @details `NA` values in a `field` column or a `burn` vector are skipped: their geometry burns
#' nothing, but still counts toward the extent when it is derived from `data`.
#'
#' @return A `terra::SpatRaster` when `encoding = "dense"`, or a `SparseArray` in COOrdinate format when
#' `encoding = "sparse"`. The raster is always 3-dimensional (band, row, col), even with a single band.
#' @export
rusterize <- function(
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
) {
  if (!inherits(data, c("sf", "sfc"))) {
    stop("`data` must be a `sf` object of type `sf` or `sfc`.")
  }

  if (!is.null(like) && !inherits(like, "SpatRaster")) {
    stop("`like` must be a `terra::SpatRaster` object.")
  }

  if (!encoding %in% c("dense", "sparse")) {
    stop("`encoding` must be one of `dense` or `sparse`.")
  }

  if (!is.null(burn) && !is.null(field)) {
    stop("Only one of `field` or `burn` can be specified.")
  }

  if (!is.null(out_shape) && !is.null(resolution)) {
    stop("`out_shape` and `resolution` are mutually exclusive; provide only one.")
  }

  if (length(burn) > 1 && length(burn) != NROW(data)) {
    stop("If `burn` is a vector of numeric values, it must have the same length as `data`.")
  }

  epsg <- sf::st_crs(data)$epsg
  if (is.na(epsg)) {
    epsg <- NULL
  }
  bounds <- NULL
  res <- NULL
  shape <- NULL

  for (nm in c("resolution", "out_shape", "extent")) {
    v <- get(nm)
    if (!is.null(v)) {
      if (!is.list(v) && !is.numeric(v)) {
        stop(sprintf("`%s` must be a numeric vector or list.", nm))
      }
      assign(nm, as.list(v))
    }
  }

  if (!is.null(like)) {
    if (!is.null(resolution) || !is.null(out_shape) || !is.null(extent)) {
      stop("`like` is mutually exclusive with `resolution`, `out_shape`, and `extent`.")
    }

    bounds <- terra::ext(like)
    bounds <- list(bounds[1], bounds[3], bounds[2], bounds[4])
    shape <- as.list(dim(like)[1:2])
  } else {
    if (is.null(resolution) && is.null(out_shape) && is.null(extent)) {
      stop("One of `resolution`, `out_shape`, or `extent` must be provided.")
    }

    if (!is.null(extent)) {
      if (is.null(resolution) && is.null(out_shape)) {
        stop("Must also specify `resolution` or `out_shape` with `extent`.")
      }
      if (length(extent) != 4 || all(unlist(extent) == 0)) {
        stop("`extent` must be a list of (xmin, ymin, xmax, ymax).")
      }
      bounds <- extent
    }

    if (!is.null(resolution)) {
      if (length(resolution) != 2 || !is.numeric(unlist(resolution)) || any(unlist(resolution) <= 0)) {
        stop("`resolution` must be 2 positive numbers.")
      }
      res <- resolution
    }

    if (!is.null(out_shape)) {
      if (length(out_shape) != 2 || any(unlist(out_shape) <= 0)) {
        stop("`out_shape` must be 2 positive numbers (nrows, ncols).")
      }
      shape <- out_shape
    }
  }

  field <- if (!is.null(field)) {
    if (!(field %in% names(data))) {
      stop(sprintf("Column `%s` not found in dataframe.", field))
    }
    data[[field]]
  }
  by <- if (!is.null(by)) {
    if (!(by %in% names(data))) {
      stop(sprintf("Column `%s` not found in dataframe.", by))
    }
    # integer codes + `levels` in first-seen order. NAs stay NA and are rejected in Rust.
    # match() instead of factor() because it converts every value to a string first
    x <- data[[by]]
    levs <- NULL
    if (is.factor(x)) {
      levs <- levels(x)
      x <- as.integer(x)
    }
    u <- unique(x[!is.na(x)])
    structure(match(x, u), levels = as.character(if (is.null(levs)) u else levs[u]))
  }

  raw_raster_info <- list(shape = shape, extent = bounds, resolution = res, tap = tap, epsg = epsg)

  geometry <- sf::st_geometry(data)

  r_array <- rusterize_r(
    sf::st_as_binary(geometry),
    raw_raster_info,
    fun,
    background,
    all_touched,
    encoding,
    dtype,
    field,
    by,
    burn
  )

  if (encoding == "dense") {
    return(.array_to_rast(r_array$to_raster(), r_array))
  }

  # rewire to_raster() for SparseArray to build a terra::rast object instead of returning a plain array.
  to_raster_fn <- r_array$to_raster
  assign("to_raster", function() .array_to_rast(to_raster_fn(), r_array), envir = r_array)
  r_array
}

# Build a terra::rast from a SparseArray or DenseArray
.array_to_rast <- function(array, r_array) {
  shape <- dim(array)
  dim(array) <- NULL
  extent <- r_array$extent()
  out <- terra::rast(
    nrows = shape[2],
    ncols = shape[1],
    nlyrs = shape[3],
    xmin = extent[1],
    xmax = extent[3],
    ymin = extent[2],
    ymax = extent[4],
    resolution = r_array$resolution(),
    names = r_array$names()
  )
  # set values after the names because assigning names on a raster that already holds values triggers a full copy
  terra::values(out) <- array
  epsg <- r_array$epsg()
  if (!is.na(epsg)) {
    terra::crs(out) <- paste0("EPSG:", epsg)
  }
  out
}
