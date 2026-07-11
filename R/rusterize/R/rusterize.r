#' Extremely fast rasterization engine built in Rust for R
#'
#' @param data An `sf` or `sfc` object of geometries.
#' @param like A `terra` template used as a spatial blueprint (resolution, dimension, extent).
#' Mutually exclusive with `resolution`, `out_shape`, and `extent`.
#' @param resolution Pixel resolution as `c(xres, yres)` (or a list).
#' @param out_shape Output raster dimensions as `c(nrows, ncols)` (or a list).
#' @param extent Spatial bounding box as `c(xmin, ymin, xmax, ymax)` (or a list).
#' @param field Column name to use for pixel values. Mutually exclusive with `burn`.
#' @param by Column used for grouping. Each group is rasterized into a distinct band in the output.
#' @param burn A static value or a list of values to apply to each geometries. If a vector, it must match the
#' length of the geometry data. Mutually exclusive with `field`. If a vector, its dtype should match the output dtype,
#' else it will be internally casted.
#' @param fun Pixel function to use when burning geometries. Available options: `sum`, `first`, `last`, `min`, `max`,
#' `count`, or `any`.
#' @param background Value assigned to pixels not covered by any geometry. Defaults to NA.
#' @param encoding The format of the returned object: `"dense"` or `"sparse"`.
#' @param all_touched If True, every pixel touched by a geometry is burned.
#' @param tap Target Aligned Pixels: aligns the extent to the pixel resolution.
#' @param dtype Output data type: `"integer"` or `"double"`.
#'
#' @return A `terra` object or a `SparseArray` in COOrdinate format.
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
  if (!("sf" %in% class(data)) && !("sfc" %in% class(data))) {
    stop("`data` must be a `sf` object of type `sf` or `sfc`.")
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

  n_geom <- if ("sf" %in% class(data)) nrow(data) else length(data)
  if (length(burn) > 1 && length(burn) != n_geom) {
    stop("If `burn` is a vector of numeric values, it must have the same length as `data`.")
  }

  epsg <- sf::st_crs(data)$epsg
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

  field <- if (!is.null(field)) data[[field]]
  by <- if (!is.null(by)) as.character(data[[by]])

  raw_raster_info <- list(shape = shape, extent = bounds, resolution = res, tap = tap, epsg = as.integer(epsg))

  geometry <- if ("sf" %in% class(data)) {
    if (!("geometry" %in% names(data))) {
      stop("No geometry column found for sf object.")
    }
    data$geometry
  } else {
    data
  }

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

  # rewire to_raster() for SparseArray and DenseArray to build a terra::rast object instead of returning a plain array.
  to_raster_fn <- r_array$to_raster
  assign("to_raster", function() .array_to_rast(to_raster_fn(), r_array), envir = r_array)
  r_array
}

# Build a terra::rast from a SparseArray or DenseArray
.array_to_rast <- function(array, r_array) {
  shape <- dim(array)
  extent <- r_array$extent()
  out <- terra::rast(
    nrows = shape[1],
    ncols = shape[2],
    nlyrs = shape[3],
    xmin = extent[1],
    xmax = extent[3],
    ymin = extent[2],
    ymax = extent[4],
    resolution = r_array$resolution(),
    names = r_array$names(),
    vals = array
  )
  terra::crs(out) <- paste0("EPSG:", r_array$epsg())
  out
}
