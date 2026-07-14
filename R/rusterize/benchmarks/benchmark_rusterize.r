library(sf)
library(raster)
library(fasterize)
library(microbenchmark)
library(rusterize)

# polygon data only
large <- st_read("../../python/benchmarks/data/canvec_50K_BC_Hydro/waterbody_2.shp", quiet = TRUE)
small <- large[1:1000, ]

fn_r <- function(v) {
  rusterize(v, resolution = c(1 / 6, 1 / 6), field = "perm")
}

fn_f <- function(v) {
  r <- raster(v, res = 1 / 6)
  fasterize(v, r, field = "perm")
}

microbenchmark(
  rusterize_large_f64 = f <- fn_r(large),
  rusterize_small_f64 = f <- fn_r(small),
  fasterize_large_f64 = f <- fn_f(large),
  fasterize_small_f64 = f <- fn_f(small),
  times = 10L,
  unit = "s"
)
