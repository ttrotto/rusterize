poly4 <- function() sf::st_sf(value = 7, grp = "a", geometry = sf::st_sfc(sq(0, 0, 4, 4), crs = 4326))

# left/right 2x2 columns tiling (0,0)-(4,4), distinct value + group per part
two_grp <- function() {
  sf::st_sf(value = c(10, 20), grp = c("a", "b"), geometry = sf::st_sfc(sq(0, 0, 2, 4), sq(2, 0, 4, 4), crs = 4326))
}

test_that("dense encoding returns a SpatRaster with correct geometry", {
  r <- rusterize(poly4(), out_shape = c(4, 4), field = "value", background = 0)
  expect_s4_class(r, "SpatRaster")
  expect_equal(terra::nlyr(r), 1)
  expect_equal(dim(r)[1:2], c(4, 4))
  expect_equal(unname(as.vector(terra::ext(r))), c(0, 4, 0, 4))
  expect_equal(terra::res(r), c(1, 1))
  expect_identical(terra::crs(r, describe = TRUE)$code, "4326")
  expect_true(all(terra::values(r) == 7))
})

test_that("sparse encoding returns an RArray whose to_raster() is a SpatRaster", {
  r <- rusterize(
    poly4(),
    out_shape = c(4, 4),
    field = "value",
    background = 0,
    encoding = "sparse"
  )
  expect_s3_class(r, "RArray")
  rast <- r$to_raster()
  expect_s4_class(rast, "SpatRaster")
  expect_true(all(terra::values(rast) == 7))
})

test_that("dense and sparse produce identical rasters", {
  args <- list(two_grp(), out_shape = c(4, 4), field = "value", background = 0)
  d <- do.call(rusterize, c(args, encoding = "dense"))
  s <- do.call(rusterize, c(args, encoding = "sparse"))$to_raster()
  expect_equal(terra::values(d), terra::values(s))
  expect_equal(as.vector(terra::ext(d)), as.vector(terra::ext(s)))
})

test_that("by grouping yields one named band per group", {
  r <- rusterize(
    two_grp(),
    out_shape = c(4, 4),
    field = "value",
    by = "grp",
    background = 0
  )
  expect_s4_class(r, "SpatRaster")
  expect_equal(terra::nlyr(r), 2)
  expect_identical(names(r), c("a", "b"))
  expect_true(all(terra::values(r[["a"]], mat = FALSE) %in% c(0, 10)))
  expect_true(all(terra::values(r[["b"]], mat = FALSE) %in% c(0, 20)))
})

test_that("burn writes a constant value across coverage", {
  r <- rusterize(poly4(), out_shape = c(4, 4), burn = 1, background = 0)
  expect_s4_class(r, "SpatRaster")
  expect_true(all(terra::values(r) == 1))
})

test_that("a like= template sets extent, resolution, and CRS", {
  tmpl <- terra::rast(nrows = 4, ncols = 4, xmin = 0, xmax = 4, ymin = 0, ymax = 4, crs = "EPSG:4326")
  r <- rusterize(poly4(), like = tmpl, field = "value", background = 0)
  expect_s4_class(r, "SpatRaster")
  expect_equal(as.vector(terra::ext(r)), as.vector(terra::ext(tmpl)))
  expect_equal(terra::res(r), terra::res(tmpl))
})

test_that("resolution alone infers the grid from geometry", {
  r <- rusterize(poly4(), resolution = c(1, 1), burn = 1, background = 0)
  expect_s4_class(r, "SpatRaster")
  expect_true(all(dim(r)[1:2] > 0))
})

test_that("out_shape alone infers resolution from geometry", {
  r <- rusterize(poly4(), out_shape = c(4, 4), burn = 1, background = 0)
  expect_equal(dim(r)[1:2], c(4, 4))
})

test_that("a vector burn matching nrow is accepted", {
  r <- rusterize(two_grp(), out_shape = c(4, 4), burn = c(3, 4), background = 0)
  expect_s4_class(r, "SpatRaster")
  expect_true(all(terra::values(r) %in% c(0, 3, 4)))
})

test_that("a bare sfc (no attribute table) works with burn", {
  r <- rusterize(make_sfc(), out_shape = c(4, 4), burn = 1, background = 0)
  expect_s4_class(r, "SpatRaster")
  expect_true(all(terra::values(r) == 1))
})
