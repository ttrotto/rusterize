library(sf)
library(raster)
library(fasterize)
library(microbenchmark)

# polygon data only
large <- st_read("canvec_50K_BC_Hydro/waterbody_2.shp", quiet = TRUE)
small <- large[1:1000, ]

fn <- function(v) {
  r <- raster(v, res = 1 / 6)
  fasterize(v, r)
}

microbenchmark(
  fasterize_large = f <- fn(large),
  fasterize_small = f <- fn(small),
  times = 10L,
  unit = "s"
)
