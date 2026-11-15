from __future__ import annotations

import importlib.metadata
from types import NoneType
from typing import TYPE_CHECKING, Literal, overload

import numpy as np

from ._dependencies import (
    _check_for_geopandas,
    _check_for_polars_st,
    _xarray_available,
)
from ._dependencies import geopandas as gpd
from ._dependencies import polars as pl
from ._dependencies import xarray as xr
from ._rusterize import _rusterize

if TYPE_CHECKING:
    from ._rusterize import SparseArray

__version__ = importlib.metadata.version("rusterize")


def _strict_astype(a: np.ndarray, dtype: str) -> np.ndarray:
    """Check for numpy-path cast to avoid burning values that would naturally fall out of range for the dtype."""

    dt = np.dtype(dtype)
    if a.dtype == dt:
        return a
    if dt.kind in "iu" and a.size:
        info = np.iinfo(dt)
        if not (a.min() >= info.min and a.max() <= info.max):
            raise ValueError(f"`burn`/`field` has values that cannot be represented as {dt} (out of range or NaN).")
    return a.astype(dt)


@overload
def rusterize(
    data: gpd.GeoDataFrame | gpd.GeoSeries | pl.DataFrame | list | np.ndarray,
    like: xr.DataArray | xr.Dataset | None = ...,
    res: tuple | list | None = ...,
    out_shape: tuple | list | None = ...,
    extent: tuple | list | None = ...,
    field: str | None = ...,
    by: str | None = ...,
    burn: int | float | np.ndarray | None = ...,
    fun: str = ...,
    background: int | float | None = ...,
    encoding: Literal["xarray"] = ...,
    all_touched: bool = ...,
    tap: bool = ...,
    dtype: str = ...,
) -> xr.DataArray: ...


@overload
def rusterize(
    data: gpd.GeoDataFrame | gpd.GeoSeries | pl.DataFrame | list | np.ndarray,
    like: xr.DataArray | xr.Dataset | None = ...,
    res: tuple | list | None = ...,
    out_shape: tuple | list | None = ...,
    extent: tuple | list | None = ...,
    field: str | None = ...,
    by: str | None = ...,
    burn: int | float | np.ndarray | None = ...,
    fun: str = ...,
    background: int | float | None = ...,
    encoding: Literal["numpy"] = ...,
    all_touched: bool = ...,
    tap: bool = ...,
    dtype: str = ...,
) -> np.ndarray: ...


@overload
def rusterize(
    data: gpd.GeoDataFrame | gpd.GeoSeries | pl.DataFrame | list | np.ndarray,
    like: xr.DataArray | xr.Dataset | None = ...,
    res: tuple | list | None = ...,
    out_shape: tuple | list | None = ...,
    extent: tuple | list | None = ...,
    field: str | None = ...,
    by: str | None = ...,
    burn: int | float | np.ndarray | None = ...,
    fun: str = ...,
    background: int | float | None = ...,
    encoding: Literal["sparse"] = ...,
    all_touched: bool = ...,
    tap: bool = ...,
    dtype: str = ...,
) -> SparseArray: ...


def rusterize(
    data: gpd.GeoDataFrame | gpd.GeoSeries | pl.DataFrame | list | np.ndarray,
    like: xr.DataArray | xr.Dataset | None = None,
    res: tuple | list | None = None,
    out_shape: tuple | list | None = None,
    extent: tuple | list | None = None,
    field: str | None = None,
    by: str | None = None,
    burn: int | float | np.ndarray | None = None,
    fun: str = "last",
    background: int | float | None = np.nan,
    encoding: Literal["xarray", "numpy", "sparse"] = "xarray",
    all_touched: bool = False,
    tap: bool = False,
    dtype: str = "float64",
) -> xr.DataArray | np.ndarray | SparseArray:
    """
    Parameters
    ----------
    data : geopandas.GeoDataFrame, geopandas.GeoSeries, polars.DataFrame, list, numpy.ndarray
        Input data to rasterize.

        - If polars.DataFrame, it must have a "geometry" column with geometries stored in WKB or WKT format.
        - If list or numpy.ndarray, geometries must be in WKT, WKB, or shapely formats (EPSG is not inferred and defaults to None).
    like : xarray.DataArray or xarray.Dataset (default: None)
        Template array used as a spatial blueprint (resolution, shape, extent). Mutually exclusive with `res`, `out_shape`, and `extent`. Requires xarray and rioxarray.
    res : tuple or list (default: None)
        Pixel resolution defined as (xres, yres). Mutually exclusive with `out_shape`.
    out_shape : tuple or list (default: None)
        Output raster dimensions defined as (nrows, ncols). Mutually exclusive with `res`.
    extent : `tuple` or `list` (default: None)
        Spatial bounding box defined as (xmin, ymin, xmax, ymax).
    field : `str` (default: None)
        Column name to use for pixel values. Mutually exclusive with `burn`. Not considered when input is list or numpy.ndarray.
        Values are cast to the output `dtype` like `burn`.
    by : `str` (default: None)
        Column used for grouping. Each group is rasterized into a distinct band in the output. Not considered when input is list or numpy.ndarray.
    burn : `int`, `float`, or `numpy.ndarray` (default: None)
        A static value or an array of values, one per geometry. If a `numpy.ndarray`, it must match the length of the geometry data. Mutually exclusive with `field`.
        Values are cast to the output `dtype`; a value that cannot be represented in it (out of range, or NaN for an integer dtype) raises an error.
        If `data` is a `geopandas.GeoSeries`, its index is used as `burn` value, unless otherwise specified.
    fun : `str` (default: "last")
        Pixel function to use when burning geometries. Available options: `sum`, `first`, `last`, `min`, `max`, `count`, or `any`.
        `sum` and `count` add each pixel at most once per geometry (like GDAL's `MERGE_ALG=ADD`), even where a line crosses itself.
        With integer dtypes they wrap around on overflow (e.g. 300 overlaps with `uint8` give 44), so pick a dtype wide enough for the expected totals.
    background : `int` or `float` (default: numpy.nan)
        Value assigned to pixels not covered by any geometry. A value that is invalid for the output `dtype` (e.g. NaN for `uint8`)
        falls back to that dtype's default (0).
    encoding : `str` (default: "xarray")
        The format of the returned object: `"xarray"`, `"numpy"`, or `"sparse"`.
    all_touched : `bool` (default: False)
        If True, every pixel touched by a geometry is burned.
    tap : `bool` (default: False)
        Target Aligned Pixels: aligns the extent to the pixel resolution.
    dtype : `str` (default: "float64")
        Output data type: `uint8`, `uint16`, `uint32`, `uint64`, `int8`, `int16`, `int32`, `int64`, `float32`, or `float64`.

    Returns
    -------
        xarray.DataArray, numpy.ndarray, or a sparse array in COO format.

    Notes
    ------
    If `encoding` is "numpy" or input is list or numpy.ndarray, the return array is without any spatial reference.

    When any of `res`, `out_shape`, or `extent` is not provided, it is inferred from the other arguments when applicable.
    If `like` is specified, `res`, `out_shape`, and `extent` are inferred from the `like` DataArray or Dataset.
    Unless `extent` is specified, a half-pixel buffer is applied to avoid missing points on the border.
    The logics dictating the final spatial properties of the rasterized geometries follow those of GDAL.

    If neither `field` nor `burn` is given, a value of 1 is burned.

    NaN or null values in a `field` column or a `burn` array are skipped: their geometry burns nothing, but still counts toward the extent when it is
    inferred from `data`. Null values in the `by` column raise an error; drop them first.
    """

    if isinstance(data, (list, np.ndarray)):
        data_type = "raw"
    elif _check_for_geopandas(data) and isinstance(data, gpd.GeoSeries):
        if data.empty:
            raise ValueError("Input data is empty.")
        data_type = "geoseries"
    elif _check_for_geopandas(data) and isinstance(data, gpd.GeoDataFrame):
        if data.empty:
            raise ValueError("Input data is empty.")
        data_type = "geopandas"
    elif _check_for_polars_st(data) and isinstance(data, pl.DataFrame):
        if data.is_empty():
            raise ValueError("Input data is empty.")
        data_type = "polars"
    else:
        raise TypeError(
            "`data` must be either geopandas.GeoDataFrame, geopandas.GeoSeries, polars.DataFrame, list, or numpy.ndarray"
        )

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

    if not isinstance(burn, (int, float, np.ndarray, NoneType)):
        raise TypeError("`burn` must be an integer, float, or a numpy.ndarray.")

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

    if encoding not in ["xarray", "numpy", "sparse"]:
        raise ValueError("`encoding` must be one of `xarray`, 'numpy', or `sparse`.")

    if encoding == "xarray" and not _xarray_available():
        raise ModuleNotFoundError(
            "`xarray` and `rioxarray` must be installed if encoding is `xarray`. Install with `pip install xarray rioxarray`."
        )

    if field and burn is not None:
        raise ValueError("Only one of `field` or `burn` can be specified.")

    if isinstance(burn, np.ndarray) and burn.size != len(data):
        raise ValueError("If `burn` is a `numpy.ndarray`, it must have the same length as `data`.")

    _bounds = None
    _res = None
    _shape = None

    if like is not None:
        if not (_xarray_available() and isinstance(like, (xr.DataArray, xr.Dataset))):
            raise TypeError("`like` must be a xarray.DataArray or xarray.Dataset")

        if any((res, out_shape, extent)):
            raise ValueError("`like` is mutually exclusive with `res`, `out_shape`, and `extent`.")

        if not hasattr(like, "rio"):
            raise AttributeError("The `like` object must have a 'rio' accessor.")

        try:
            # extent + shape fully determines resolution; passing res too trips the shape/res mutex
            if isinstance(like, xr.Dataset):
                _shape = tuple(like.sizes[d] for d in ("y", "x"))
                if not _shape:
                    raise ValueError("`like` object has no known spatial dimensions.")
            else:
                _shape = like.squeeze().shape
            _bounds = like.rio.bounds()
        except Exception as e:
            raise AttributeError("No spatial dimension found for like object") from e
    else:
        if not res and not out_shape and not extent:
            raise ValueError("One of `res`, `out_shape`, or `extent` must be provided.")

        if res and out_shape:
            raise ValueError("`res` and `out_shape` are mutually exclusive; provide only one.")

        if extent:
            if not res and not out_shape:
                raise ValueError("Must also specify `res` or `out_shape` with extent.")

            if len(extent) != 4 or all(e == 0 for e in extent):
                raise ValueError("`extent` must be a tuple or list of (xmin, ymin, xmax, ymax).")
            _bounds = extent

        if res:
            if len(res) != 2 or any(r <= 0 for r in res) or any(not isinstance(r, (int, float)) for r in res):
                raise ValueError("`res` must be 2 positive numbers.")
            _res = res

        if out_shape:
            if len(out_shape) != 2 or any(s <= 0 for s in out_shape) or any(not isinstance(s, int) for s in out_shape):
                raise ValueError("`out_shape` must be 2 positive integers.")
            _shape = out_shape

    # extract columns of interest, if any
    column = None
    epsg = None

    # `by` goes to Rust as integer codes + one label per group
    by_codes = None
    by_labels = None

    # data-specific feature extraction
    match data_type:
        case "geopandas":
            epsg = data.crs.to_epsg() if data.crs else None

            try:
                if field:
                    burn = data[field].to_numpy()
                if by:
                    by_col = data[by]
            except KeyError as e:
                raise KeyError("Column not found in GeoDataFrame.") from e

            if by:
                # nulls are -1, which wraps to an out-of-range uint32
                codes, labels = by_col.factorize()
                by_codes = codes.astype(np.uint32)
                by_labels = labels.astype(str).tolist()

            # plain numpy array to match GeoSeries construct
            geometries = np.asarray(data.geometry)

        case "polars":
            # check if geometry has SRID. If 0, then None, else assume first SRID is equal for all geometries
            try:
                srid = data.select(pl.col("geometry").first().st.srid()).item()
            except pl.exceptions.ColumnNotFoundError as e:
                raise ValueError("If `polars.DataFrame`, a 'geometry' column is expected.") from e

            epsg = None if srid == 0 else srid

            try:
                if field:
                    column = data.get_column(field)
                if by:
                    by_col = data.get_column(by)
                    if by_col.dtype.is_float():
                        by_col = by_col.fill_nan(None)
                    by_col = by_col.cast(pl.String)
            except pl.exceptions.ColumnNotFoundError as e:
                raise KeyError("Column not found in polars DataFrame.") from e

            if by:
                # Enum codes index the labels in first-seen order
                labels = by_col.unique(maintain_order=True).drop_nulls()
                by_codes = by_col.cast(pl.Enum(labels)).to_physical().cast(pl.UInt32).fill_null(len(labels)).to_numpy()
                by_labels = labels.to_list()

            # geometries are extracted directly on the Rust side
            geometries = data.select(pl.col("geometry")).to_series()

        case "geoseries":
            # plain numpy array: avoids GeoSeries.geom_type, which computes the type of every row
            geometries = np.asarray(data.geometry)
            burn = burn if burn is not None else data.index.to_numpy()

            try:
                epsg = data.crs.to_epsg()
            except AttributeError:
                pass

        case _:
            # list or numpy.ndarray
            geometries = data

    # check that burn matches the output dtype
    if isinstance(burn, np.ndarray):
        burn = _strict_astype(burn, dtype)

    # RawRasterInfo
    raw_raster_info = {
        "shape": _shape,
        "extent": _bounds,
        "resolution": _res,
        "tap": tap,
        "epsg": epsg,
    }

    return _rusterize(
        geometries,
        raw_raster_info,
        fun,
        column,
        by_codes,
        by_labels,
        burn,
        background,
        all_touched,
        encoding,
        dtype,
    )
