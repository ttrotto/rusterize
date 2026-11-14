"""Benchmark rusterize against GDAL, rasterio, and geocube in a single table.

Run from anywhere:

    pytest python/benchmarks/benchmark_rusterize.py --benchmark-min-rounds=10 --benchmark-time-unit=s

The datasets (~1.3 GB) are downloaded into `benchmarks/data` on the first run.
GDAL >= 3.11 is required (in-memory vector datasets). rasterio and geocube benchmarks are
skipped when those packages are not installed.
"""

import math
import zipfile
from io import BytesIO
from pathlib import Path

import numpy as np
import pytest
import requests
from osgeo import gdal
from pyogrio import read_dataframe
from rusterize import rusterize

DATA = Path(__file__).parent / "data"
WATER = DATA / "canvec_50K_BC_Hydro" / "waterbody_2.shp"
ROADS = DATA / "lrnf000r25p_e" / "lrnf000r25p_e.gpkg"

# degrees, ~185 m x 105 m at 55°N
WATER_RES = 1 / 600
# metres
ROADS_RES = 100
N_SMALL = 1000


def _download(url: str, members: str | None = None) -> None:
    response = requests.get(url, timeout=600)
    response.raise_for_status()
    with zipfile.ZipFile(BytesIO(response.content), "r") as zip_ref:
        names = [n for n in zip_ref.namelist() if members is None or members in n]
        zip_ref.extractall(DATA, members=names)


# POLYGONS (~468 MB)
if not WATER.exists():
    _download(
        "https://ftp.maps.canada.ca/pub/nrcan_rncan/vector/canvec/shp/Hydro/canvec_50K_BC_Hydro_shp.zip",
        members="canvec_50K_BC_Hydro/waterbody_2",
    )

# LINESTRINGS (~900 MB)
if not ROADS.exists():
    _download("https://www12.statcan.gc.ca/census-recensement/2011/geo/RNF-FRR/files-fichiers/lrnf000r25p_e.zip")

water_large = read_dataframe(WATER)
water_small = water_large.iloc[:N_SMALL, :]
roads = read_dataframe(ROADS)

# geocube rasterizes every numeric column by default; give it a single column of ones so it burns
# the same single band of 1s as the other tools
geocube_water_large = water_large[["geometry"]].assign(value=1.0)
geocube_water_small = water_small[["geometry"]].assign(value=1.0)

# copy GDAL sources into in-memory datasets so feature decoding happens up front
gdal.UseExceptions()
_mem = gdal.GetDriverByName("MEM")
gdal_water_large = _mem.CreateCopy("", gdal.OpenEx(str(WATER)))
gdal_roads = _mem.CreateCopy("", gdal.OpenEx(str(ROADS)))

gdal_water_small = _mem.Create("", 0, 0, 0, gdal.GDT_Unknown)
_src_layer = gdal_water_large.GetLayer(0)
_dst_layer = gdal_water_small.CreateLayer(_src_layer.GetName(), _src_layer.GetSpatialRef(), _src_layer.GetGeomType())
for _i, _feat in enumerate(_src_layer):
    if _i >= N_SMALL:
        break
    _dst_layer.CreateFeature(_feat)
_src_layer.ResetReading()


def gdal_rasterize(src, res, output_type):
    return gdal.Rasterize("", src, xRes=res, yRes=res, format="MEM", outputType=output_type, burnValues=1)


def rasterio_grid(gdf, res):
    """Shape and transform covering the data bounds, as a rasterio user would build them."""
    from affine import Affine

    xmin, ymin, xmax, ymax = gdf.total_bounds
    shape = (math.ceil((ymax - ymin) / res), math.ceil((xmax - xmin) / res))
    return shape, Affine(res, 0, xmin, 0, -res, ymax)


# RUSTERIZE
def test_water_small_rusterize_f64(benchmark):
    benchmark(rusterize, water_small, res=(WATER_RES, WATER_RES), dtype="float64")


def test_water_small_rusterize_f64_numpy(benchmark):
    benchmark(rusterize, water_small, res=(WATER_RES, WATER_RES), dtype="float64", encoding="numpy")


def test_water_large_rusterize_f64(benchmark):
    benchmark(rusterize, water_large, res=(WATER_RES, WATER_RES), dtype="float64")


def test_water_large_rusterize_f64_numpy(benchmark):
    benchmark(rusterize, water_large, res=(WATER_RES, WATER_RES), dtype="float64", encoding="numpy")


def test_roads_rusterize_uint8(benchmark):
    benchmark(rusterize, roads, res=(ROADS_RES, ROADS_RES), dtype="uint8")


# GDAL
def test_water_small_gdal_f64(benchmark):
    benchmark(gdal_rasterize, gdal_water_small, WATER_RES, gdal.GDT_Float64)


def test_water_large_gdal_f64(benchmark):
    benchmark(gdal_rasterize, gdal_water_large, WATER_RES, gdal.GDT_Float64)


def test_roads_gdal_uint8(benchmark):
    benchmark(gdal_rasterize, gdal_roads, ROADS_RES, gdal.GDT_Byte)


# RASTERIO
def test_water_small_rasterio_f64(benchmark):
    features = pytest.importorskip("rasterio.features")
    shape, transform = rasterio_grid(water_small, WATER_RES)
    benchmark(features.rasterize, water_small.geometry, out_shape=shape, transform=transform, dtype=np.float64)


def test_water_large_rasterio_f64(benchmark):
    features = pytest.importorskip("rasterio.features")
    shape, transform = rasterio_grid(water_large, WATER_RES)
    benchmark(features.rasterize, water_large.geometry, out_shape=shape, transform=transform, dtype=np.float64)


# GEOCUBE
def test_water_small_geocube_f64(benchmark):
    core = pytest.importorskip("geocube.api.core")
    benchmark(core.make_geocube, geocube_water_small, measurements=["value"], resolution=WATER_RES)


def test_water_large_geocube_f64(benchmark):
    core = pytest.importorskip("geocube.api.core")
    benchmark(core.make_geocube, geocube_water_large, measurements=["value"], resolution=WATER_RES)
