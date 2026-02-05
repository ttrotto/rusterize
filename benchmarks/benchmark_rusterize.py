import zipfile
from io import BytesIO

import requests
from pyogrio import read_dataframe
from rusterize import rusterize

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


# BENCHES
def test_water_large(benchmark):
    benchmark(rusterize, water_large, res=(1 / 6, 1 / 6), dtype="uint8")


def test_water_small(benchmark):
    benchmark(rusterize, water_small, res=(1 / 6, 1 / 6), dtype="uint8")


def test_roads(benchmark):
    benchmark(rusterize, roads, res=(50, 50), dtype="uint8")
