from __future__ import annotations

import importlib.metadata
from types import NoneType
from typing import TYPE_CHECKING

import numpy as np

from ._dependencies import (
    _check_for_geopandas,
    _check_for_polars_st,
    _polars_available,
    _xarray_available,
)
from ._dependencies import geopandas as gpd
from ._dependencies import polars as pl
from ._dependencies import xarray as xr
from .rusterize import _rusterize

if TYPE_CHECKING:
    from .rusterize import SparseArray

__version__ = importlib.metadata.version("rusterize")


def rusterize(
    data: gpd.GeoDataFrame | pl.DataFrame | list | np.ndarray,
    like: xr.DataArray | xr.Dataset | None = None,
    res: tuple | list | None = None,
    out_shape: tuple | list | None = None,
    extent: tuple | list | None = None,
    field: str | None = None,
    by: str | None = None,
    burn: int | float | None = None,
    fun: str = "last",
    background: int | float | None = np.nan,
    encoding: str = "xarray",
    all_touched: bool = False,
    tap: bool = False,
    dtype: str = "float64",
) -> xr.DataArray | np.ndarray | SparseArray:
    """
    Fast geometry rasterization in Rust.

    Parameters
    ----------
    data : geopandas.GeoDataFrame, polars.DataFrame, list, numpy.ndarray
      Input data to rasterize.
      - If polars.DataFrame, it must be have a "geometry" column with geometries stored in WKB or WKT format.
      - If list or numpy.ndarray, geometries must be in WKT, WKB, or shapely formats (EPSG is not inferred and defaults to None).
    like : xarray.DataArray or xarray.Dataset (default: None)
      Template array used as a spatial blueprint (resolution, shape, extent). Mutually exclusive with `res`, `out_shape`, and `extent`. Requires xarray and rioxarray.
    res : tuple or list (default: None)
      Pixel resolution defined as (xres, yres).
    out_shape : tuple or list (default: None)
      Output raster dimensions defined as (nrows, ncols).
    extent : `tuple` or `list` (default: None)
      Spatial bounding box defined as `(xmin, ymin, xmax, ymax)`.
    field : `str` (default: None)
      Column name to use for pixel values. Mutually exclusive with `burn`. Not considered when input is list or numpy.ndarray.
    by : `str` (default: None)
      Column used for grouping. Each group is rasterized into a distinct band in the output. Not considered when input is list or numpy.ndarray.
    burn : `int` or `float` (default: None)
      A static value to apply to all geometries. Mutually exclusive with `field`.
    fun : `str` (default: "last")
      Pixel function to use when burning geometries. Available options: `sum`, `first`, `last`, `min`, `max`, `count`, or `any`.
    background : `int` or `float` (default: numpy.nan)
      Value assigned to pixels not covered by any geometry.
    encoding : `str` (default: "xarray")
      The format of the returned object: `"xarray"`, `"numpy"`, or `"sparse"`.
    all_touched : `bool` (default: False)
      If True, every pixel touched by a geometry is burned.
    tap : `bool` (default: False)
      Target Aligned Pixels: aligns the extent to the pixel resolution.
    dtype : `str` (default: "float64")
      Output data type (e.g., `uint8`, `int32`, `float32`).

    Returns
    -------
        xarray.DataArray, numpy.ndarray, or a sparse array in COO format.

    Notes
    -----
        If `encoding` is "numpy" or input is list or numpy.ndarray, the return array is without any spatial reference.

        When any of `res`, `out_shape`, or `extent` is not provided, it is inferred from the other arguments when applicable.
        If `like` is specified, `res`, `out_shape`, and `extent` are inferred from the `like` DataArray or Dataset.
        Unless `extent` is specified, a half-pixel buffer is applied to avoid missing points on the border.
        The logics dictating the final spatial properties of the rasterized geometries follow those of GDAL.

        If `field` is not in `data`, then a default `burn` value of 1 is rasterized.

        A `None` value for `dtype` corresponds to the default of that dtype. An illegal value for a dtype will be replaced with the default of that dtype. For example, a `background=np.nan` for `dtype="uint8"` will become `background=0`, where `0` is the default for `uint8`.
    """
    data_type = None
    if isinstance(data, (list, np.ndarray)):
        data_type = "raw"
    elif _check_for_geopandas(data) and isinstance(data, gpd.GeoDataFrame):
        if data.empty:
            raise ValueError("GeoDataFrame is empty.")
        data_type = "geopandas"
    elif _check_for_polars_st(data) and isinstance(data, pl.DataFrame):
        if data.is_empty():
            raise ValueError("GeoDataFrame is empty.")
        data_type = "polars"
    else:
        raise TypeError("`data` must be either geopandas.GeoDataFrame, polars.DataFrame, list, or numpy.ndarray")

    if not isinstance(res, (tuple, list, NoneType)):
        raise TypeError("`resolution` must be a tuple or list of (xres, yres).")

    if not isinstance(out_shape, (tuple, list, NoneType)):
        raise TypeError("`out_shape` must be a tuple or list of (nrows, ncols).")

    if not isinstance(extent, (tuple, list, NoneType)):
        raise TypeError("`extent` must be a tuple or list of (xmin, ymin, xmax, ymax).")

    if not isinstance(field, (str, NoneType)):
        raise TypeError("`field` must be a string column name.")

    if not isinstance(by, (str, NoneType)):
        raise TypeError("`by` must be a string column name.")

    if not isinstance(burn, (int, float, NoneType)):
        raise TypeError("`burn` must be an integer or float.")

    if not isinstance(fun, str):
        raise TypeError("`pixel_fn` must be one of sum, first, last, min, max, count, or any.")

    if not isinstance(background, (int, float, NoneType)):
        raise TypeError("`background` must be integer, float, or None.")

    if not isinstance(encoding, str):
        raise TypeError("`encoding` must be one of 'xarray', 'numpy', or 'sparse'.")

    if not isinstance(all_touched, bool):
        raise TypeError("`all_touched` must be a boolean.")

    if not isinstance(tap, bool):
        raise TypeError("`tap` must be a boolean.")

    if not isinstance(dtype, str):
        raise TypeError(
            "`dtype` must be a one of 'uint8', 'uint16', 'uint32', 'uint64', 'int8', 'int16', 'int32', 'int64', 'float32', 'float64'"
        )

    # value checks
    if field and burn:
        raise ValueError("Only one of `field` or `burn` can be specified.")

    if encoding not in ["xarray", "numpy", "sparse"]:
        raise ValueError("`encoding` must be one of `xarray`, 'numpy', or `sparse`.")

    if encoding == "xarray" and not _xarray_available():
        raise ModuleNotFoundError(
            "`xarray` and `rioxarray` must be installed if encoding is `xarray`. Install with `pip install xarray rioxarray`."
        )

    _with_user_extent = False
    _bounds = (np.inf, np.inf, np.inf, np.inf)
    _res = (0, 0)
    _shape = (0, 0)

    if like is not None:
        if not (_xarray_available() and isinstance(like, (xr.DataArray, xr.Dataset))):
            raise TypeError("`like` must be a xarray.DataArray or xarray.Dataset")

        if any((res, out_shape, extent)):
            raise ValueError("`like` is mutually exclusive with `res`, `out_shape`, and `extent`.")

        if not hasattr(like, "rio"):
            raise AttributeError("The `like` object must have a 'rio' accessor.")

        try:
            affine = like.rio.transform()
            _res = (affine.a, abs(affine.e))
            _shape = like.squeeze().shape
            _bounds, _with_user_extent = like.rio.bounds(), True
        except Exception as e:
            raise AttributeError("No spatial dimension found for like object") from e
    else:
        if not res and not out_shape and not extent:
            raise ValueError("One of `res`, `out_shape`, or `extent` must be provided.")

        if extent:
            if not res and not out_shape:
                raise ValueError("Must also specify `res` or `out_shape` with extent.")

            if len(extent) != 4 or all(e == 0 for e in extent):
                raise ValueError("`extent` must be a tuple or list of (xmin, ymin, xmax, ymax).")

            _bounds = extent
            _with_user_extent = True

        if res:
            if len(res) != 2 or any(r <= 0 for r in res) or any(not isinstance(r, (int, float)) for r in res):
                raise ValueError("`res` must be 2 positive numbers.")

            _res = res

        if out_shape:
            if len(out_shape) != 2 or any(s <= 0 for s in out_shape) or any(not isinstance(s, int) for s in out_shape):
                raise ValueError("`out_shape` must be 2 positive integers.")

            _shape = out_shape

        # extract columns of interest if dataframe
        cols = list(set([col for col in (field, by) if col and col != "geometry"]))
        df = None
        epsg = None

        # get bounds
        if data_type == "geopandas":
            if not _polars_available():
                raise ModuleNotFoundError("polars must be installed when data is geopandas.GeoDataFrame.")

            if not _with_user_extent:
                _bounds = data.total_bounds

            epsg = data.crs.to_epsg() if data.crs else None

            if cols:
                try:
                    df = pl.from_pandas(data[cols])
                except KeyError as e:
                    raise KeyError("Column not found in GeoDataFrame.") from e

            geometries = data.geometry

        elif data_type == "polars":
            if not _with_user_extent:
                try:
                    _bounds = data.select(pl.col("geometry").st.total_bounds()).item().to_numpy()
                except pl.exceptions.ColumnNotFoundError as e:
                    raise ValueError("If `polars.DataFrame`, a 'geometry' column is expected.") from e

            # check if geometry has SRID. If 0, then None, else assume first SRID is equal for all geometries
            srid = data.select(pl.col("geometry").first().st.srid()).item()
            epsg = None if srid == 0 else srid

            if cols:
                try:
                    df = data.select(pl.col([*cols, "geometry"]))
                except pl.exceptions.ColumnNotFoundError as e:
                    raise KeyError("Column not found in polars DataFrame.") from e

            # geometries are extracted directly on the Rust side
            geometries = data.select(pl.col("geometry")).to_series()

        else:
            # list or numpy.ndarray
            geometries = data

    # RawRasterInfo
    raw_raster_info = {
        "nrows": _shape[0],
        "ncols": _shape[1],
        "xmin": _bounds[0],
        "ymin": _bounds[1],
        "xmax": _bounds[2],
        "ymax": _bounds[3],
        "xres": _res[0],
        "yres": _res[1],
        "with_user_extent": _with_user_extent,
        "tap": tap,
        "epsg": epsg,
    }

    return _rusterize(
        geometries,
        raw_raster_info,
        fun,
        df,
        field,
        by,
        burn,
        background,
        all_touched,
        encoding,
        dtype,
    )
