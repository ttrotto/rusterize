from osgeo import gdal, ogr

import os
import re
import warnings
from tempfile import NamedTemporaryFile
from unittest.mock import patch

import geopandas as gpd
import numpy as np
import polars_st as st
import pytest
import xarray as xr
from rusterize import rusterize
from shapely import wkt

gdal.UseExceptions()

GEOMS = [
    "POLYGON ((-180 -20, -140 55, 10 0, -140 -60, -180 -20), (-150 -20, -100 -10, -110 20, -150 -20))",
    "POLYGON ((-10 0, 140 60, 160 0, 140 -55, -10 0))",
    "POLYGON ((-125 0, 0 60, 40 5, 15 -45, -125 0))",
    "MULTILINESTRING ((-180 -70, -140 -50), (-140 -50, -100 -70), (-100 -70, -60 -50), (-60 -50, -20 -70), (-20 -70, 20 -50), (20 -50, 60 -70), (60 -70, 100 -50), (100 -50, 140 -70), (140 -70, 180 -50))",
    "GEOMETRYCOLLECTION (POINT (50 -40), POLYGON ((75 -40, 75 -30, 100 -30, 100 -40, 75 -40)), LINESTRING (60 -40, 80 0), GEOMETRYCOLLECTION (POLYGON ((100 20, 100 30, 110 30, 110 20, 100 20))))",
]

geometries = [wkt.loads(geom) for geom in GEOMS]
GDF = gpd.GeoDataFrame({"value": range(1, len(GEOMS) + 1)}, geometry=geometries)


@pytest.fixture(scope="module")
def exploded_gpkg():
    """Temporary GPKG with exploded geometries for GDAL"""
    with NamedTemporaryFile(suffix=".gpkg", delete=False) as tmp:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            # GDAL struggles with nested collections, so we explode all of them to be safe
            GDF.explode().explode().to_file(tmp.name, driver="GPKG", layer="test")
        path = tmp.name
    yield path

    if os.path.exists(path):
        os.remove(path)


class TestTypeChecks:
    @pytest.mark.parametrize(
        "kwargs, expected_match",
        [
            ({"data": "not_a_dataframe", "res": (1, 1)}, "`data` must be either geopandas"),
            ({"like": "not_an_xarray", "res": (1, 1)}, "`like` must be a xarray.DataArray"),
            ({"res": "1x1"}, "`resolution` must be a tuple or list"),
            ({"out_shape": "100x100"}, "`out_shape` must be a tuple or list"),
            ({"extent": "0,0,10,10"}, "`extent` must be a tuple or list"),
            ({"field": 123}, "`field` must be a string"),
            ({"by": 123}, "`by` must be a string"),
            ({"burn": "hot"}, "`burn` must be an integer, float"),
            ({"fun": 1}, "`pixel_fn` must be one of"),
            ({"background": "black"}, "`background` must be integer, float, or None"),
            ({"encoding": 1}, "`encoding` must be one of 'xarray'"),
            ({"all_touched": "yes"}, "`all_touched` must be a boolean"),
            ({"tap": "yes"}, "`tap` must be a boolean"),
            ({"dtype": 64}, "`dtype` must be a one of"),
        ],
    )
    def test_type_errors(self, kwargs, expected_match):
        args = {"data": GDF, "res": (1, 1)}
        args.update(kwargs)

        with pytest.raises(TypeError, match=expected_match):
            rusterize(**args)


class TestMissingDependencies:
    def test_geopandas_missing(self):
        import geopandas as gpd
        from shapely import wkt

        gdf = gpd.GeoDataFrame(geometry=wkt.loads(GEOMS))

        with patch("rusterize._check_for_geopandas", return_value=False):
            with pytest.raises(TypeError, match="`data` must be either geopandas.GeoDataFrame"):
                rusterize(gdf, res=(1, 1), encoding="numpy")

    def test_polars_st_missing(self):
        import polars_st as st

        plst = st.GeoDataFrame({"geometry": GEOMS})

        with patch("rusterize._check_for_polars_st", return_value=False):
            with pytest.raises(TypeError, match="`data` must be either geopandas.GeoDataFrame, geopandas.GeoSeries, polars.DataFrame"):
                rusterize(plst, res=(1, 1), encoding="numpy")

    def test_xarray_encoding_missing(self):
        with patch("rusterize._xarray_available", return_value=False):
            with pytest.raises(ModuleNotFoundError, match="`xarray` and `rioxarray` must be installed"):
                rusterize(GEOMS, res=(1, 1), encoding="xarray")

    def test_xarray_like_missing(self):
        with patch("rusterize._xarray_available", return_value=False):
            import xarray as xr
            like = xr.DataArray()

            with pytest.raises(TypeError, match="`like` must be a xarray.DataArray or xarray.Dataset"):
                rusterize(GEOMS, like=like, encoding="numpy")


class TestArguments:
    def test_burn_parameter(self):
        r = rusterize(GDF, res=(1, 1), burn=99, encoding="numpy").squeeze()
        assert np.nanmax(r) == 99
        assert np.nanmin(r[r > 0]) == 99

    def test_background_parameter(self):
        bg_value = -1
        r = rusterize(GDF, res=(1, 1), burn=1, background=bg_value, encoding="numpy").squeeze()
        assert r[0, 0] == bg_value

    def test_mutually_exclusive_field_burn(self):
        with pytest.raises(ValueError, match="Only one of `field` or `burn` can be specified"):
            rusterize(GDF, res=(1, 1), field="value", burn=5)

    def test_burn_array_length_mismatch(self):
        expected_msg = "If `burn` is a `numpy.ndarray`, it must have the same length as `data`."
        with pytest.raises(ValueError, match=re.escape(expected_msg)):
            rusterize(GDF, res=(1, 1), burn=np.array([1, 2]))

    def test_empty_data_error(self):
        with pytest.raises(ValueError, match="Input data is empty."):
            rusterize(gpd.GeoDataFrame(geometry=[]), res=(1, 1))

    def test_missing_spatial_metadata_error(self):
        with pytest.raises(ValueError, match="One of `res`, `out_shape`, or `extent` must be provided"):
            rusterize(GDF)

    def test_invalid_resolution_error(self):
        with pytest.raises(ValueError, match="`res` must be 2 positive numbers"):
            rusterize(GDF, res=(-1, 1))

    def test_invalid_shape_error(self):
        with pytest.raises(ValueError, match="`out_shape` must be 2 positive integers"):
            rusterize(GDF, out_shape=(-1, 1))

    def test_invalid_extent_error1(self):
        with pytest.raises(ValueError, match="Must also specify `res` or `out_shape` with extent."):
            rusterize(GDF, extent=(1, 2, 3, 4))

    def test_invalid_extent_error2(self):
        expected_msg = "`extent` must be a tuple or list of (xmin, ymin, xmax, ymax)."
        with pytest.raises(ValueError, match=re.escape(expected_msg)):
            rusterize(GDF, res=(1, 1), extent=(0, 0, 0, 0))

    def test_mutually_exclusive_like(self):
        like = rusterize(GDF, res=(1, 1), field="value", encoding="xarray")
        with pytest.raises(ValueError, match="`like` is mutually exclusive with `res`, `out_shape`, and `extent`."):
            rusterize(GDF, like=like, res=(1, 1))


class TestFormats:
    def test_inputs(self):
        # geopandas
        r_gpd = rusterize(GDF, res=(1, 1), dtype="uint8", fun="sum", encoding="numpy")

        # geoseries
        r_gs = rusterize(GDF.geometry, res=(1, 1), dtype="uint8", fun="sum", encoding="numpy", burn=1)

        # list of shapely geometries
        r_list_geom = rusterize(list(GDF.geometry), res=(1, 1), dtype="uint8", fun="sum", encoding="numpy", burn=1)

        # list or numpy WKT
        r_list = rusterize(GEOMS, res=(1, 1), dtype="uint8", fun="sum", encoding="numpy")
        r_numpy = rusterize(np.asarray(GEOMS), res=(1, 1), dtype="uint8", fun="sum", encoding="numpy")

        # list or numpy WKB
        r_list_wkb = rusterize(GDF.to_wkb().geometry.tolist(), res=(1, 1), dtype="uint8", fun="sum", encoding="numpy")
        r_numpy_wkb = rusterize(
            np.asarray(GDF.to_wkb().geometry), res=(1, 1), dtype="uint8", fun="sum", encoding="numpy"
        )

        # polars ST WKT
        plst = st.GeoDataFrame({"value": list(range(1, len(GEOMS) + 1)), "geometry": GEOMS})
        r_plst = rusterize(plst, res=(1, 1), dtype="uint8", fun="sum", encoding="numpy")

        # polars ST WKB
        plst_wkb = plst.st.to_wkb()
        r_plst_wkb = rusterize(plst_wkb, res=(1, 1), dtype="uint8", fun="sum", encoding="numpy")

        assert np.allclose(r_gpd, r_list)
        assert np.allclose(r_gpd, r_list_geom)
        assert np.allclose(r_gpd, r_gs)
        assert np.allclose(r_gpd, r_numpy)
        assert np.allclose(r_gpd, r_plst)
        assert np.allclose(r_gpd, r_list_wkb)
        assert np.allclose(r_gpd, r_numpy_wkb)
        assert np.allclose(r_gpd, r_plst_wkb)

    def test_geoseries_burn_input(self):
        burn = np.arange(1, len(GEOMS) + 1)
        r_burn = rusterize(GEOMS, res=(1, 1), dtype="uint8", burn=burn, fun="sum", encoding="numpy")
        r_field = rusterize(GDF, res=(1, 1), dtype="uint8", field="value", fun="sum", encoding="numpy")
        assert np.allclose(r_burn, r_field)

    def test_burn_array(self):
        burn = np.arange(1, len(GEOMS) + 1)
        r_burn = rusterize(GDF, res=(1, 1), dtype="uint8", burn=burn, fun="sum", encoding="numpy")
        r_field = rusterize(GDF, res=(1, 1), dtype="uint8", field="value", fun="sum", encoding="numpy")
        assert np.allclose(r_burn, r_field)

    def test_outputs(self):
        r_numpy = rusterize(GDF, res=(1, 1), dtype="uint8", field="value", encoding="numpy")
        r_xarray = rusterize(GDF, res=(1, 1), dtype="uint8", field="value")
        r_sparse1 = rusterize(GDF, res=(1, 1), dtype="uint8", field="value", encoding="sparse").to_numpy()
        r_sparse2 = rusterize(GDF, res=(1, 1), dtype="uint8", field="value", encoding="sparse").to_xarray()

        assert np.allclose(r_numpy, r_xarray.data)
        assert np.allclose(r_numpy, r_sparse1)
        assert np.allclose(r_numpy, r_sparse2.data)


class TestCoherence:
    def test_standard(self):
        # comparing against a known-good static file
        r = rusterize(GDF, res=(1, 1), dtype="uint8", field="value", fun="sum", encoding="numpy").squeeze()

        data_path = "test/data/standard_output_sum.tif"
        with gdal.Open(data_path) as src:
            gdal_array = src.ReadAsArray()
            assert np.allclose(r, gdal_array)

    def test_alltouched(self, exploded_gpkg):
        src_gdal = gdal.OpenEx(exploded_gpkg)
        out_ds = gdal.Rasterize(
            "",
            src_gdal,
            format="MEM",
            outputType=gdal.GDT_Byte,
            xRes=0.5,
            yRes=0.5,
            attribute="value",
            layers=["test"],
            allTouched=True,
            add=True,
        )

        r = rusterize(
            GDF.explode().explode(),
            res=(0.5, 0.5),
            dtype="uint8",
            field="value",
            fun="sum",
            encoding="numpy",
            all_touched=True,
        ).squeeze()

        assert np.allclose(r, out_ds.ReadAsArray())


class TestGrouping:
    def test_by_category_matches_text(self):
        # `by` reaches Rust as codes + labels, so a categorical column must group exactly like its text
        gdf = GDF.assign(group=["a", "b", "a", "b", "a"])
        text = rusterize(gdf, res=(1, 1), field="value", by="group", encoding="numpy")
        cat = rusterize(gdf.assign(group=gdf["group"].astype("category")), res=(1, 1), field="value", by="group", encoding="numpy")
        assert text.shape[0] == 2
        assert np.array_equal(text, cat, equal_nan=True)

    def test_by_nulls_raise(self):
        groups = ["a", None, "a", "b", "a"]
        with pytest.raises(RuntimeError, match="Found nulls in `by` column"):
            rusterize(GDF.assign(group=groups), res=(1, 1), by="group", encoding="numpy")
        with pytest.raises(RuntimeError, match="Found nulls in `by` column"):
            rusterize(st.GeoDataFrame({"geometry": GEOMS, "group": groups}), res=(1, 1), by="group", encoding="numpy")


class TestCustomRaster:
    def test_like(self):
        like = rusterize(GDF, res=(1, 1), dtype="uint8", field="value", encoding="xarray")
        r = rusterize(GDF, like=like, dtype="uint8", field="value", encoding="numpy").squeeze()
        assert r.shape == like.squeeze().shape

    def test_extent_standard(self, exploded_gpkg):
        extent = [-349, -507, 1, 0]
        src_gdal = gdal.OpenEx(exploded_gpkg)
        out_ds = gdal.Rasterize(
            "",
            src_gdal,
            format="MEM",
            outputType=gdal.GDT_Byte,
            xRes=1,
            yRes=1,
            outputBounds=extent,
            attribute="value",
            layers=["test"],
            add=True,
        )

        r = rusterize(
            GDF.explode().explode(),
            res=(1, 1),
            dtype="uint8",
            field="value",
            extent=extent,
            fun="sum",
            encoding="numpy",
        ).squeeze()

        assert np.allclose(r, out_ds.ReadAsArray())

    def test_extent_alltouched(self, exploded_gpkg):
        extent = [-349, -507, 1, 0]
        src_gdal = gdal.OpenEx(exploded_gpkg)
        out_ds = gdal.Rasterize(
            "",
            src_gdal,
            format="MEM",
            outputType=gdal.GDT_Byte,
            xRes=1,
            yRes=1,
            outputBounds=extent,
            attribute="value",
            layers=["test"],
            allTouched=True,
            add=True,
        )

        r = rusterize(
            GDF.explode().explode(),
            res=(1, 1),
            dtype="uint8",
            field="value",
            extent=extent,
            fun="sum",
            encoding="numpy",
            all_touched=True,
        ).squeeze()

        assert np.allclose(r, out_ds.ReadAsArray())

    def test_shape_standard(self, exploded_gpkg):
        shape = (47, 319)  # (height, width)

        # interestingly, GDAL cuts the end/start of the lines with this custom shape
        data_path = "test/data/standard_output_sum_custom_shape.tif"
        with gdal.Open(data_path) as src:
            gdal_array = src.ReadAsArray()

        r = rusterize(
            GDF.explode().explode(),
            dtype="uint8",
            field="value",
            out_shape=shape,
            fun="sum",
            encoding="numpy",
        ).squeeze()

        assert np.allclose(r, gdal_array)

    def test_shape_alltouched(self, exploded_gpkg):
        shape = (47, 319)  # (height, width)
        src_gdal = gdal.OpenEx(exploded_gpkg)
        out_ds = gdal.Rasterize(
            "",
            src_gdal,
            format="MEM",
            outputType=gdal.GDT_Byte,
            width=shape[1],
            height=shape[0],
            attribute="value",
            layers=["test"],
            allTouched=True,
            add=True,
        )

        r = rusterize(
            GDF.explode().explode(),
            dtype="uint8",
            field="value",
            out_shape=shape,
            fun="sum",
            encoding="numpy",
            all_touched=True,
        ).squeeze()

        assert np.allclose(r, out_ds.ReadAsArray())

    def test_some_user_inputs_standard(self, exploded_gpkg):
        # GDAL doesn't directly support res + shape as input parameters here
        extent = [-349, -507, 1, 0]
        shape = (47, 319)

        src_gdal = gdal.OpenEx(exploded_gpkg)
        out_ds = gdal.Rasterize(
            "",
            src_gdal,
            format="MEM",
            outputType=gdal.GDT_Byte,
            width=shape[1],
            height=shape[0],
            outputBounds=extent,
            attribute="value",
            layers=["test"],
            add=True,
        )

        r = rusterize(
            GDF.explode().explode(),
            dtype="uint8",
            field="value",
            out_shape=shape,
            extent=extent,
            fun="sum",
            encoding="numpy",
        ).squeeze()

        assert np.allclose(r, out_ds.ReadAsArray())

    def test_some_user_inputs_alltouched(self, exploded_gpkg):
        # GDAL doesn't directly support res + shape as input parameters here
        extent = [-349, -507, 1, 0]
        shape = (47, 319)

        src_gdal = gdal.OpenEx(exploded_gpkg)
        out_ds = gdal.Rasterize(
            "",
            src_gdal,
            format="MEM",
            outputType=gdal.GDT_Byte,
            width=shape[1],
            height=shape[0],
            outputBounds=extent,
            attribute="value",
            layers=["test"],
            allTouched=True,
            add=True,
        )

        r = rusterize(
            GDF.explode().explode(),
            dtype="uint8",
            field="value",
            out_shape=shape,
            extent=extent,
            fun="sum",
            encoding="numpy",
            all_touched=True,
        ).squeeze()

        assert np.allclose(r, out_ds.ReadAsArray())


# Lines that exercise segment joints, self-crossings, part ends and raster edges. Reference: GDAL with MERGE_ALG=ADD.
LINE_CASES = {
    "mls_connected": "MULTILINESTRING ((0.5 9.5, 3.5 9.5), (3.5 9.5, 3.5 6.5))",
    "mls_disjoint": "MULTILINESTRING ((0.5 9.5, 3.5 9.5), (0.5 5.5, 3.5 5.5))",
    "ls_corner": "LINESTRING (0.5 9.5, 3.5 9.5, 3.5 6.5)",
    "ls_self_crossing": "LINESTRING (0.5 0.5, 8.5 8.5, 8.5 0.5, 0.5 8.5)",
    "ls_diagonal_offset": "LINESTRING (0.2 0.7, 9.3 6.1)",
    "mls_connected_offset": "MULTILINESTRING ((0.2 0.7, 4.1 3.3), (4.1 3.3, 9.6 9.1))",
    "ls_closed_ring": "LINESTRING (1.5 1.5, 7.5 1.5, 7.5 7.5, 1.5 7.5, 1.5 1.5)",
    "ls_degenerate": "LINESTRING (2.5 2.5, 2.5 2.5)",
    "ls_on_pixel_boundary": "LINESTRING (1 5, 6 5)",
    "ls_exit_reenter": "LINESTRING (2.5 2.5, 14.5 6.5, 7.5 8.5)",
    "mls_chain": "MULTILINESTRING ((0.5 0.5, 3.2 4.7), (3.2 4.7, 6.9 2.1), (6.9 2.1, 9.4 9.4))",
    "mls_cross": "MULTILINESTRING ((0.5 0.5, 9.5 9.5), (0.5 9.5, 9.5 0.5))",
}


def gdal_sum(wkt_geom: str, xres: float, yres: float, all_touched: bool) -> np.ndarray:
    """Burn 1 with MERGE_ALG=ADD onto a 10x10 world-unit grid"""
    ds = gdal.GetDriverByName("MEM").Create("", round(10 / xres), round(10 / yres), 1, gdal.GDT_Int32)
    ds.SetGeoTransform((0, xres, 0, 10, 0, -yres))
    src = gdal.GetDriverByName("MEM").Create("", 0, 0, 0, gdal.GDT_Unknown)
    layer = src.CreateLayer("lines")
    feature = ogr.Feature(layer.GetLayerDefn())
    feature.SetGeometry(ogr.CreateGeometryFromWkt(wkt_geom))
    layer.CreateFeature(feature)
    gdal.RasterizeLayer(ds, [1], layer, burn_values=[1], options=["MERGE_ALG=ADD", f"ALL_TOUCHED={all_touched}"])
    return ds.ReadAsArray()


class TestLines:
    @pytest.mark.parametrize("all_touched", [False, True])
    @pytest.mark.parametrize("res", [(1, 1), (1, 0.5), (0.5, 1)], ids=["square", "tall", "wide"])
    @pytest.mark.parametrize("name", LINE_CASES)
    def test_sum_matches_gdal(self, name, res, all_touched):
        # each pixel counts once per geometry, whatever the pixel shape
        wkt_geom = LINE_CASES[name]
        xres, yres = res
        r = rusterize(
            [wkt_geom],
            res=res,
            extent=(0, 0, 10, 10),
            fun="sum",
            dtype="int32",
            background=0,
            all_touched=all_touched,
            encoding="numpy",
        ).squeeze()
        assert np.array_equal(r, gdal_sum(wkt_geom, xres, yres, all_touched))


class TestValues:
    def test_nan_field_values_are_skipped(self):
        # second polygon overlaps the first and has a NaN value: it must not burn
        square = "POLYGON ((0 0, 4 0, 4 4, 0 4, 0 0))"
        values = [5.0, np.nan]
        gdf = gpd.GeoDataFrame({"value": values}, geometry=[wkt.loads(square)] * 2)
        plst = st.GeoDataFrame({"value": values, "geometry": [square] * 2})
        for data in (gdf, plst):
            for fun in ("last", "sum"):
                r = rusterize(data, res=(1, 1), field="value", fun=fun, background=0, encoding="numpy")
                assert not np.isnan(r).any() and np.max(r) == 5.0, f"{type(data).__name__} {fun}"

    @pytest.mark.parametrize("burn", [np.array([1.0, np.nan, 1.0, 1.0, 1.0]), np.array([1, 300, 1, 1, 1])])
    def test_unrepresentable_burn_values_raise(self, burn):
        with pytest.raises(ValueError, match="cannot be represented as uint8"):
            rusterize(GDF, res=(1, 1), burn=burn, dtype="uint8", encoding="numpy")

    def test_invalid_dtype_raises(self):
        with pytest.raises(ValueError, match="Invalid dtype"):
            rusterize(GDF, res=(1, 1), dtype="int128", encoding="numpy")
