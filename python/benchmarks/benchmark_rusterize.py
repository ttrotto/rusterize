import os
import zipfile
from io import BytesIO

import requests
from osgeo import gdal
from pyogrio import read_dataframe
from rusterize import rusterize
import rioxarray

# POLYGONS (~468MB)
url = "https://ftp.maps.canada.ca/pub/nrcan_rncan/vector/canvec/shp/Hydro/canvec_50K_BC_Hydro_shp.zip"
response = requests.get(url)

with zipfile.ZipFile(BytesIO(response.content), "r") as zip_ref:
    for file_name in zip_ref.namelist():
        if "canvec_50K_BC_Hydro/waterbody_2" in file_name:
            zip_ref.extract(file_name)


water_large = read_dataframe("canvec_50K_BC_Hydro/waterbody_2.shp")
water_small = water_large.iloc[:1000, :]


# LINESTRINGS (~900 MB)
url = "https://www12.statcan.gc.ca/census-recensement/2011/geo/RNF-FRR/files-fichiers/lrnf000r25p_e.zip"
response = requests.get(url)

with zipfile.ZipFile(BytesIO(response.content), "r") as zip_ref:
    zip_ref.extractall()

roads = read_dataframe("lrnf000r25p_e/lrnf000r25p_e.gpkg")


# Copy GDAL sources into in-memory datasets so feature decoding happens up front
gdal.UseExceptions()
_mem = gdal.GetDriverByName("Memory")
src_water = _mem.CreateCopy("", gdal.OpenEx("canvec_50K_BC_Hydro/waterbody_2.shp"))
src_roads = _mem.CreateCopy("", gdal.OpenEx("lrnf000r25p_e/lrnf000r25p_e.gpkg"))


src_water_small = _mem.Create("", 0, 0, 0, gdal.GDT_Unknown)
_src_layer = src_water.GetLayer(0)
_dst_layer = src_water_small.CreateLayer(
    _src_layer.GetName(), _src_layer.GetSpatialRef(), _src_layer.GetGeomType()
)
for _i, _feat in enumerate(_src_layer):
    if _i >= 1000:
        break
    _dst_layer.CreateFeature(_feat)
_src_layer.ResetReading()


# BENCHES
def test_water_large_f64(benchmark):
    benchmark(rusterize, water_large, res=(1 / 6, 1 / 6), dtype="float64")


def test_water_small_f64(benchmark):
    benchmark(rusterize, water_small, res=(1 / 6, 1 / 6), dtype="float64")


def test_water_large_f64_numpy(benchmark):
    benchmark(rusterize, water_large, res=(1 / 6, 1 / 6), dtype="float64", encoding="numpy")


def test_water_small_f64_numpy(benchmark):
    benchmark(rusterize, water_small, res=(1 / 6, 1 / 6), dtype="float64", encoding="numpy")


def test_roads_uint8(benchmark):
    benchmark(rusterize, roads, res=(50, 50), dtype="uint8")


def test_water_large_gdal_f64(benchmark):
    benchmark(
        gdal.Rasterize,
        "",
        src_water,
        xRes=1 / 6,
        yRes=1 / 6,
        format="MEM",
        outputType=gdal.GDT_Float64,
        burnValues=1,
    )


def test_water_small_gdal_f64(benchmark):
    benchmark(
        gdal.Rasterize,
        "",
        src_water_small,
        xRes=1 / 6,
        yRes=1 / 6,
        format="MEM",
        outputType=gdal.GDT_Float64,
        burnValues=1,
    )


def test_roads_gdal_uint8(benchmark):
    benchmark(
        gdal.Rasterize,
        "",
        src_roads,
        xRes=50,
        yRes=50,
        format="MEM",
        outputType=gdal.GDT_Byte,
        burnValues=1,
    )
