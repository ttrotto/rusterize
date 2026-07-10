# Shared functions for rusterize tests.

# A single 4x4 square polygon (0,0)-(4,4) as an sf data.frame with a numeric
# `value` field and a character `grp` grouping column
make_poly <- function(crs = 4326) {
  sf::st_sf(
    value = 2,
    grp = "a",
    geometry = sf::st_sfc(sf::st_polygon(list(rbind(c(0, 0), c(4, 0), c(4, 4), c(0, 4), c(0, 0)))), crs = crs)
  )
}

# Two adjacent 2x2 squares, distinct fields/groups. Left occupies x in [0,2],
# right x in [2,4]; both span y in [0,4]. Together they tile (0,0)-(4,4)
make_two_polys <- function(crs = 4326) {
  left <- sf::st_polygon(list(rbind(c(0, 0), c(2, 0), c(2, 4), c(0, 4), c(0, 0))))
  right <- sf::st_polygon(list(rbind(c(2, 0), c(4, 0), c(4, 4), c(2, 4), c(2, 0))))
  sf::st_sf(value = c(10, 20), grp = c("a", "b"), geometry = sf::st_sfc(left, right, crs = crs))
}

# WKB encoding of a geometry column
wkb <- function(x) sf::st_as_binary(if (inherits(x, "sf")) x$geometry else x)

# Single-band matrix view of a RArray
r_mat <- function(r) r$to_raster()[,, 1]

# RawRasterInfo
raster_info <- function() {
  list(shape = list(4, 4), extent = list(0, 0, 4, 4), tap = FALSE, epsg = 4326L)
}

# A square
sq <- function(x0, y0, x1, y1) {
  sf::st_polygon(list(rbind(c(x0, y0), c(x1, y0), c(x1, y1), c(x0, y1), c(x0, y0))))
}

# Bare sfc
make_sfc <- function(crs = 4326) {
  sf::st_sfc(sf::st_polygon(list(rbind(c(0, 0), c(4, 0), c(4, 4), c(0, 4), c(0, 0)))), crs = crs)
}
