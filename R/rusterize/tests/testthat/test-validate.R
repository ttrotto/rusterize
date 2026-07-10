test_that("data must be sf or sfc", {
  expect_error(rusterize(data.frame(x = 1), resolution = c(1, 1), burn = 1), "must be a", fixed = TRUE)
})

test_that("encoding must be dense or sparse", {
  expect_error(
    rusterize(make_poly(), resolution = c(1, 1), burn = 1, encoding = "bogus"),
    "encoding` must be one of",
    fixed = TRUE
  )
})

test_that("field and burn are mutually exclusive", {
  expect_error(rusterize(make_poly(), resolution = c(1, 1), field = "value", burn = 1), "Only one of", fixed = TRUE)
})

test_that("vector burn must match nrow(data)", {
  expect_error(rusterize(make_poly(), resolution = c(1, 1), burn = c(1, 2, 3)), "same length as", fixed = TRUE)
})

test_that("like is mutually exclusive with resolution/out_shape/extent", {
  tmpl <- terra::rast(nrows = 4, ncols = 4, xmin = 0, xmax = 4, ymin = 0, ymax = 4)
  expect_error(rusterize(make_poly(), like = tmpl, resolution = c(1, 1), burn = 1), "mutually exclusive", fixed = TRUE)
})

test_that("at least one of resolution/out_shape/extent is required", {
  expect_error(rusterize(make_poly(), burn = 1), "must be provided", fixed = TRUE)
})

test_that("extent requires resolution or out_shape", {
  expect_error(rusterize(make_poly(), extent = c(0, 0, 4, 4), burn = 1), "Must also specify", fixed = TRUE)
})

test_that("extent must have length 4 and not be all zero", {
  expect_error(
    rusterize(make_poly(), extent = c(0, 0, 4), resolution = c(1, 1), burn = 1),
    "must be a list of",
    fixed = TRUE
  )
  expect_error(
    rusterize(make_poly(), extent = c(0, 0, 0, 0), resolution = c(1, 1), burn = 1),
    "must be a list of",
    fixed = TRUE
  )
})

test_that("resolution must be 2 positive numbers", {
  expect_error(rusterize(make_poly(), resolution = c(1), burn = 1), "2 positive numbers", fixed = TRUE)
  expect_error(rusterize(make_poly(), resolution = c(-1, 1), burn = 1), "2 positive numbers", fixed = TRUE)
})

test_that("out_shape must be 2 positive numbers", {
  expect_error(rusterize(make_poly(), out_shape = c(4), burn = 1), "2 positive numbers", fixed = TRUE)
  expect_error(rusterize(make_poly(), out_shape = c(0, 4), burn = 1), "2 positive numbers", fixed = TRUE)
})
