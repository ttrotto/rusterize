from __future__ import annotations

import importlib.metadata
from types import NoneType
from typing import List, Tuple

import numpy as np
import polars as pl
import rioxarray
from geopandas import GeoDataFrame
from xarray import DataArray, Dataset

from .rusterize import _rusterize

__version__ = importlib.metadata.version("rusterize")


def rusterize(
    gdf: GeoDataFrame,
    like: DataArray | Dataset | None = None,
    res: Tuple | List | None = None,
    out_shape: Tuple | List | None = None,
    extent: Tuple | List | None = None,
    field: str | None = None,
    by: str | None = None,
    burn: int | float | None = None,
    fun: str = "last",
    background: int | float | None = np.nan,
    dtype: str = "float64",
) -> DataArray:
    """
    Fast geopandas rasterization into xarray.DataArray

    Args:
        :param gdf: geopandas dataframe to rasterize.
        :param like: array to use as blueprint for spatial matching (resolution, shape, extent). Mutually exlusive with res, out_shape, and extent.
        :param res: (xres, yres) for rasterized data.
        :param out_shape: (nrows, ncols) for regularized output shape.
        :param extent: (xmin, xmax, ymin, ymax) for regularized extent.
        :param field: field to rasterize, mutually exclusive with `burn`. Default is None.
        :param by: column to rasterize, assigns each unique value to a layer in the stack based on field. Default is None.
        :param burn: burn a value onto the raster, mutually exclusive with `field`. Default is None.
        :param fun: pixel function to use. Available options are `sum`, `first`, `last`, `min`, `max`, `count`, or `any`. Default is `last`.
        :param background: background value in final raster. Default is np.nan.
        :param dtype: specify the output dtype. Default is `float64`.

    Returns:
        Rasterized xarray.DataArray.

    Notes:
        When any of `res`, `out_shape`, or `extent` is not provided, it is inferred from the other arguments when applicable.
        If `like` is specified, `res`, `out_shape`, and `extent` are inferred from the `like` DataArray.
        Unless `extent` is specified, a half-pixel buffer is applied to avoid missing points on the border.
        The logics dictating the final spatial properties of the rasterized geometries follow those of GDAL.

        If `field` is not in `gdf`, then a default `burn` value of 1 is rasterized.

        A `None` value for `dtype` corresponds to the default of that dtype. An illegal value for a dtype will be replaced with the default of
        that dtype. For example, a `background=np.nan` for `dtype="uint8"` will become `background=0`, where `0` is the default for `uint8`.
    """
    # type checks
    if not isinstance(gdf, GeoDataFrame):
        raise TypeError("`gdf` must be a geopandas dataframe.")
    if not isinstance(like, (DataArray, Dataset, NoneType)):
        raise TypeError("`like' must be a xarray.DataArray or xarray.Dataset")
    if not isinstance(res, (tuple, list, NoneType)):
        raise TypeError("`resolution` must be a tuple or list of (x, y).")
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
    if not isinstance(dtype, str):
        raise TypeError(
            "`dtype` must be a one of uint8, uint16, uint32, uint64, int8, int16, int32, int64, float32, float64"
        )

    # value checks and defaults
    if field and burn:
        raise ValueError("Only one of `field` or `burn` can be specified.")
    if like is not None:
        if any((res, out_shape, extent)):
            raise ValueError("`like` is mutually exclusive with `res`, `out_shape`, and `extent`.")
        else:
            affine = like.rio.transform()
            _res = (affine.a, abs(affine.e))
            _shape = like.squeeze().shape
            _bounds, _has_extent = like.rio.bounds(), True
    else:
        if not res and not out_shape and not extent:
            raise ValueError("One of `res`, `out_shape`, or `extent` must be provided.")
        if extent and not res and not out_shape:
            raise ValueError("Must also specify `res` or `out_shape` with extent.")
        if res and (len(res) != 2 or any(r <= 0 for r in res) or any(not isinstance(r, (int, float)) for r in res)):
            raise ValueError("`res` must be 2 positive numbers.")
        if out_shape and (
            len(out_shape) != 2 or any(s <= 0 for s in out_shape) or any(not isinstance(s, int) for s in out_shape)
        ):
            raise ValueError("`out_shape` must be 2 positive integers.")
        if extent and len(extent) != 4:
            raise ValueError("`extent` must be a tuple or list of (xmin, ymin, xmax, ymax).")

        # defaults
        _res = res if res else (0, 0)
        _shape = out_shape if out_shape else (0, 0)
        (_bounds, _has_extent) = (extent, True) if extent else (gdf.total_bounds, False)

    # RasterInfo
    raster_info = {
        "nrows": _shape[0],
        "ncols": _shape[1],
        "xmin": _bounds[0],
        "ymin": _bounds[1],
        "xmax": _bounds[2],
        "ymax": _bounds[3],
        "xres": _res[0],
        "yres": _res[1],
        "has_extent": _has_extent,
    }

    # extract columns of interest and convert to polars
    cols = list(set([col for col in (field, by) if col]))
    try:
        df = pl.from_pandas(gdf[cols]) if cols else None
    except KeyError as e:
        raise KeyError("Column not found in GeoDataFrame.") from e

    # rusterize
    r = _rusterize(gdf.geometry, raster_info, fun, df, field, by, burn, background, dtype)
    return DataArray.from_dict(r).rio.write_crs(gdf.crs, inplace=True)
