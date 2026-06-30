import numpy as np
from affine import Affine
from geocube.api.core import make_geocube
from pyogrio import read_dataframe
from rasterio.features import rasterize

water = read_dataframe("canvec_50K_BC_Hydro/waterbody_2.shp")

res = 1 / 6
xmin, ymin, xmax, ymax = water.total_bounds
out_shape = (75, 148)
transform = Affine(res, 0, xmin, 0, -res, ymax)


def test_rasterio(benchmark):
    benchmark(
        rasterize,
        water.geometry,
        out_shape=out_shape,
        transform=transform,
        dtype=np.float64,
    )


def test_geocube(benchmark):
    # NOTE: make_geocube builds a full georeferenced xarray Dataset over every column
    benchmark(make_geocube, water, resolution=1 / 6)
