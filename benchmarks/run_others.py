import numpy as np
from geocube.api.core import make_geocube

# from geocube.api.core import make_geocube
from pyogrio import read_dataframe
from rasterio.features import rasterize

water = read_dataframe("canvec_50K_BC_Hydro/waterbody_2.shp")


def test_rasterio(benchmark):
    benchmark(rasterize, water.geometry, out_shape=(75, 148), dtype=np.float64)


def test_geocube(benchmark):
    benchmark(make_geocube, water, resolution=1 / 6)
