full <- function() sf::st_sfc(sq(0, 0, 4, 4), crs = 4326)

test_that("any marks presence regardless of value", {
  two <- sf::st_sfc(sq(0, 0, 4, 4), sq(0, 0, 4, 4), crs = 4326)
  r <- rusterize_r(wkb(two), raster_info(), "any", 0, FALSE, "dense", "double", NULL, NULL, c(5, 7))
  expect_equal(r_mat(r), matrix(1, 4, 4))
})

test_that("points burn the containing cell", {
  geom <- sf::st_sfc(sf::st_point(c(1.5, 1.5)), crs = 4326)
  r <- rusterize_r(wkb(geom), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 9)
  m <- r_mat(r)
  expect_equal(sum(m), 9)
  expect_equal(sum(m > 0), 1)
})

test_that("linestrings burn crossed cells", {
  geom <- sf::st_sfc(sf::st_linestring(rbind(c(0, 0), c(4, 4))), crs = 4326)
  r <- rusterize_r(wkb(geom), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 9)
  m <- r_mat(r)
  expect_equal(m, matrix(c(0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 9, 0, 0, 9, 0, 0), 4, 4))
  expect_equal(sum(m > 0), 3)
})

test_that("all_touched burns more cells than center sampling", {
  geom <- sf::st_sfc(sf::st_linestring(rbind(c(0, 0), c(4, 4))), crs = 4326)
  off <- rusterize_r(wkb(geom), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 9)
  on <- rusterize_r(wkb(geom), raster_info(), "last", 0, TRUE, "dense", "double", NULL, NULL, 9)
  expect_gt(sum(r_mat(on) > 0), sum(r_mat(off) > 0))
})

test_that("multipolygons burn all parts", {
  mp <- sf::st_multipolygon(list(
    list(rbind(c(0, 0), c(2, 0), c(2, 2), c(0, 2), c(0, 0))),
    list(rbind(c(2, 2), c(4, 2), c(4, 4), c(2, 4), c(2, 2)))
  ))
  r <- rusterize_r(wkb(sf::st_sfc(mp, crs = 4326)), raster_info(), "last", 0, FALSE, "dense", "double", NULL, NULL, 9)
  expect_equal(r_mat(r), matrix(c(0, 0, 9, 9, 0, 0, 9, 9, 9, 9, 0, 0, 9, 9, 0, 0), 4, 4))
})

test_that("non-zero background fills uncovered cells", {
  geom <- sf::st_sfc(sq(0, 0, 2, 2), crs = 4326)
  r <- rusterize_r(wkb(geom), raster_info(), "last", -1, FALSE, "dense", "double", NULL, NULL, 9)
  m <- r_mat(r)
  expect_true(all(m[m != 9] == -1))
  expect_equal(sum(m == 9), 4)
})

test_that("NA background is accepted and yields a filled raster", {
  r <- rusterize_r(wkb(full()), raster_info(), "last", NA_real_, FALSE, "dense", "double", NULL, NULL, 3)
  expect_equal(r_mat(r), matrix(3, 4, 4))
})

test_that("unknown pixel function errors", {
  expect_error(
    rusterize_r(wkb(full()), raster_info(), "bogus", 0, FALSE, "dense", "double", NULL, NULL, 1),
    "pixel function",
    fixed = TRUE
  )
})

test_that("unsupported dtype/encoding errors", {
  expect_error(
    rusterize_r(wkb(full()), raster_info(), "last", 0, FALSE, "dense", "weird", NULL, NULL, 1),
    "Unsupported dtype",
    fixed = TRUE
  )
})
