# import zipfile
# from io import BytesIO

# import requests
from osgeo import gdal
from pyogrio import read_dataframe
from rusterize import rusterize

# # POLYGONS (~468MB)
# url = "https://ftp.maps.canada.ca/pub/nrcan_rncan/vector/canvec/shp/Hydro/canvec_50K_BC_Hydro_shp.zip"
# response = requests.get(url)

# with zipfile.ZipFile(BytesIO(response.content), "r") as zip_ref:
#     for file_name in zip_ref.namelist():
#         if "canvec_50K_BC_Hydro/waterbody_2" in file_name:
#             zip_ref.extract(file_name)


water_large = read_dataframe("canvec_50K_BC_Hydro/waterbody_2.shp")
water_small = water_large.iloc[:1000, :]


# # LINESTRINGS (~900 MB)
# url = "https://www12.statcan.gc.ca/census-recensement/2011/geo/RNF-FRR/files-fichiers/lrnf000r25p_e.zip"
# response = requests.get(url)

# with zipfile.ZipFile(BytesIO(response.content), "r") as zip_ref:
#     zip_ref.extractall()

roads = read_dataframe("lrnf000r25p_e/lrnf000r25p_e.gpkg")


# GDAL
gdal.UseExceptions()
src_water = gdal.OpenEx("canvec_50K_BC_Hydro/waterbody_2.shp")
dest_water = "/vsimem/output_water.tif"
src_roads = gdal.OpenEx("lrnf000r25p_e/lrnf000r25p_e.gpkg")
dest_roads = "/vsimem/output_roads.tif"


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


def test_water_large_gdal_vsimem_f64(benchmark):
    benchmark(
        gdal.Rasterize,
        dest_water,
        src_water,
        xRes=1 / 6,
        yRes=1 / 6,
        format="GTIFF",
        outputType=gdal.GDT_Float64,
        burnValues=1,
    )


def test_roads_gdal_vsimem_uint8(benchmark):
    benchmark(
        gdal.Rasterize,
        dest_roads,
        src_roads,
        xRes=50,
        yRes=50,
        format="GTIFF",
        outputType=gdal.GDT_Byte,
        burnValues=1,
    )
