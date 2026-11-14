# Benchmark rusterize against fasterize. Run from anywhere in the repo:
#
#   Rscript R/rusterize/benchmarks/benchmark_rusterize.r
#
# The water bodies data is downloaded by the Python benchmarks (python/benchmarks/benchmark_rusterize.py).

library(sf)
library(raster)
library(fasterize)
library(microbenchmark)
library(rusterize)

root <- system2("git", c("rev-parse", "--show-toplevel"), stdout = TRUE)
path <- file.path(root, "python", "benchmarks", "data", "canvec_50K_BC_Hydro", "waterbody_2.shp")
if (!file.exists(path)) {
  stop("Water bodies data not found: run the Python benchmarks once to download it.")
}

# same resolution as the Python benchmarks: degrees, ~185 m x 105 m at 55°N
res <- 1 / 600

large <- st_read(path, quiet = TRUE)
small <- large[1:1000, ]

fn_r <- function(v) {
  rusterize(v, resolution = c(res, res), field = "perm")
}

fn_f <- function(v, template = raster(v, res = res)) {
  fasterize(v, template, field = "perm")
}

# fasterize needs a template raster::RasterLayer: time it with and without building the template
template_large <- raster(large, res = res)
template_small <- raster(small, res = res)

cat("With template creation:\n")
print(microbenchmark(
  rusterize_large_f64 = fn_r(large),
  rusterize_small_f64 = fn_r(small),
  fasterize_large_f64 = fn_f(large),
  fasterize_small_f64 = fn_f(small),
  times = 10L,
  unit = "s"
))

cat("\nWithout template creation:\n")
print(microbenchmark(
  rusterize_large_f64 = fn_r(large),
  rusterize_small_f64 = fn_r(small),
  fasterize_large_f64 = fn_f(large, template_large),
  fasterize_small_f64 = fn_f(small, template_small),
  times = 10L,
  unit = "s"
))
