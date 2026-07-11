full <- function() sf::st_sfc(sq(0, 0, 4, 4), crs = 4326)

test_that("rusterize_r returns a wrapped RArray with working methods", {
  r <- rusterize_r(wkb(full()), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 1)
  expect_s3_class(r, "RArray")
  expect_identical(dim(r$to_raster()), c(4L, 4L, 1L))
  expect_identical(r$extent(), c(0, 0, 4, 4))
  expect_identical(r$resolution(), c(1, 1))
  expect_identical(r$epsg(), 4326L)
  expect_length(r$names(), 1L)
})

test_that("full-coverage burn fills every cell", {
  r <- rusterize_r(wkb(full()), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 7)
  expect_equal(r_mat(r), matrix(7, 4, 4))
})

test_that("partial coverage leaves background elsewhere", {
  geom <- sf::st_sfc(sq(0, 0, 2, 2), crs = 4326)
  r <- rusterize_r(wkb(geom), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 9)
  expect_equal(r_mat(r), matrix(c(0, 0, 9, 9, 0, 0, 9, 9, 0, 0, 0, 0, 0, 0, 0, 0), 4, 4))
})

test_that("field values are burned per geometry", {
  geom <- sf::st_sfc(sq(0, 0, 2, 4), sq(2, 0, 4, 4), crs = 4326)
  r <- rusterize_r(wkb(geom), raster_info(), "last", 0, FALSE, "dense", "double", c(10, 20), NULL, NULL)
  expect_equal(r_mat(r), matrix(c(rep(10, 8), rep(20, 8)), 4, 4))
})

test_that("pixel functions combine overlapping geometries", {
  two <- sf::st_sfc(sq(0, 0, 4, 4), sq(0, 0, 4, 4), crs = 4326)
  vals <- c(5, 7)
  px <- function(fun, v = vals) {
    r_mat(rusterize_r(wkb(two), raster_info(), fun, 0, FALSE, "dense", "double", NULL, NULL, v))
  }
  expect_equal(px("sum"), matrix(12, 4, 4))
  expect_equal(px("min"), matrix(5, 4, 4))
  expect_equal(px("max"), matrix(7, 4, 4))
  expect_equal(px("first"), matrix(5, 4, 4))
  expect_equal(px("last"), matrix(7, 4, 4))
  expect_equal(px("count", c(1, 1)), matrix(2, 4, 4))
})

test_that("dtype controls the data type of the returned array", {
  ri <- rusterize_r(wkb(full()), raster_info(), "last", 0, FALSE, "dense", "integer", NULL, NULL, 1L)
  rn <- rusterize_r(wkb(full()), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 1)
  expect_type(ri$to_raster(), "integer")
  expect_type(rn$to_raster(), "double")
})

test_that("dense and sparse encodings agree on values", {
  d <- rusterize_r(wkb(full()), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 3)
  s <- rusterize_r(wkb(full()), raster_info(), "last", 0, FALSE, "sparse", "double", NULL, NULL, 3)
  expect_equal(d$to_raster(), s$to_raster())
})

test_that("shape-only and resolution-only both derive a valid raster", {
  shape_only <- list(shape = list(4, 4), tap = FALSE, epsg = 4326L)
  res_only <- list(resolution = list(1, 1), tap = FALSE, epsg = 4326L)
  a <- rusterize_r(wkb(full()), shape_only, "last", 0, FALSE, "dense", "double", NULL, NULL, 1)
  b <- rusterize_r(wkb(full()), res_only, "last", 0, FALSE, "dense", "double", NULL, NULL, 1)
  # shape-only fits the (0,0,4,4) envelope exactly; resolution-only applies GDAL's
  # half-pixel buffer, so (0,0,4,4) at res 1 expands to (-0.5,4.5) => 5x5
  expect_identical(dim(a$to_raster()), c(4L, 4L, 1L))
  expect_identical(dim(b$to_raster()), c(5L, 5L, 1L))
})

test_that("missing CRS yields epsg 0", {
  r <- rusterize_r(
    wkb(sf::st_sfc(sq(0, 0, 4, 4))),
    list(shape = list(4, 4), tap = FALSE),
    "last",
    0,
    FALSE,
    "dense",
    "double",
    NULL,
    NULL,
    1
  )
  expect_identical(r$epsg(), 0L)
})

test_that("by grouping produces one band per group in (row, col, band) order", {
  geom <- sf::st_sfc(sq(0, 0, 2, 4), sq(2, 0, 4, 4), crs = 4326)
  r <- rusterize_r(wkb(geom), raster_info(), "last", 0, FALSE, "dense", "double", c(10, 20), c("a", "b"), NULL)
  m <- r$to_raster()
  expect_identical(dim(m), c(4L, 4L, 2L))
  expect_identical(r$names(), c("a", "b"))
  # each band equals the single-geometry rasterization of its group
  expect_equal(m[,, 1], matrix(c(rep(10, 8), rep(0, 8)), 4, 4))
  expect_equal(m[,, 2], matrix(c(rep(0, 8), rep(20, 8)), 4, 4))
})
