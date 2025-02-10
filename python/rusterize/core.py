from __future__ import annotations

from types import NoneType
from typing import Any, Dict, Optional, Tuple, Union

import polars as pl
from pandas import DataFrame
import rioxarray
from xarray import DataArray
from .rusterize import _rusterize


def rusterize(gdf: DataFrame,
              res: Optional[Union[Tuple[int, ...], Tuple[float, ...]]] = None,
              out_shape: Optional[Union[Tuple[int, ...]]] = None,
              extent: Optional[Union[Tuple[int, ...], Tuple[float, ...]]] = None,
              field: Optional[str] = None,
              by: Optional[str] = None,
              fun: str = "last",
              background: Optional[Union[int, float]] = None,
              ) -> Dict[str, Any]:
    """
    Fast geopandas rasterization into xarray.DataArray

    Args:
        :param gdf: geopandas dataframe to rasterize.
        :param res: tuple of (xres, yres) for rasterized data.
        :param out_shape: tuple of (nrows, ncols) for regularized output shape.
        :param extent: tuple of (xmin, xmax, ymin, ymax) for regularized extent.
        :param field: field to rasterize. Default is None.
        :param by: column to rasterize, assigns each unique value to a layer in the stack based on field. Default is None.
        :param fun: pixel function to use, see fasterize for options. Default is `last`.
        :param background: background value in final raster. Default is None.

    Returns:
        Rasterized xarray.DataArray.

    Note:
        When any of `res`, `out_shape`, or `extent` is not provided, it is inferred from the other arguments when applicable.
        Unless `extent` is specified, a half-pixel buffer is applied to avoid missing points on the border.
        The logics dictating the final spatial properties of the rasterized geometries follow those of GDAL.
    """
    # type checks
    if not isinstance(gdf, DataFrame):
        raise TypeError("Must pass a valid geopandas dataframe.")
    if not isinstance(res, (tuple, NoneType)):
        raise TypeError("Must pass a valid resolution tuple (x, y).")
    if not isinstance(out_shape, (tuple, NoneType)):
        raise TypeError("Must pass a valid output shape tuple (nrows, ncols).")
    if not isinstance(extent, (tuple, NoneType)):
        raise TypeError("Must pass a valid extent tuple (xmin, ymin, xmax, ymax).")
    if not isinstance(field, (str, NoneType)):
        raise TypeError("Must pass a valid string to field.")
    if not isinstance(by, (str, NoneType)):
        raise TypeError("Must pass a valid string to by.")
    if not isinstance(fun, str):
        raise TypeError("Must pass a valid string to pixel_fn. Select only of sum, first, last, min, max, count, or any.")
    if not isinstance(background, (int, float, NoneType)):
        raise TypeError("Must pass a valid background type.")

    # value check
    if not res and not out_shape and not extent:
        raise ValueError("One of `res`, `out_shape`, or `extent` must be provided.")
    if extent and not res and not out_shape:
        raise ValueError("Must also specify `res` or `out_shape` with extent.")
    if res and (len(res) != 2 or any(r <= 0 for r in res) or any(not isinstance(r, (int, float)) for r in res)):
        raise ValueError("Resolution must be 2 positive numbers.")
    if out_shape and (len(out_shape) != 2 or any(s <= 0 for s in out_shape) or any(not isinstance(s, int) for s in out_shape)):
        raise ValueError("Output shape must be 2 positive integers.")
    if extent and len(extent) != 4:
        raise ValueError("Extent must be 4 numbers (xmin, ymin, xmax, ymax).")
    if by and not field:
        raise ValueError("If by is specified, field must also be specified.")

    # defaults
    _res = res if res else (0, 0)
    _shape = out_shape if out_shape else (0, 0)
    (_bounds, has_extent) = (extent, True) if extent else (gdf.total_bounds, False)

    # RasterInfo
    raster_info = {
        "xmin": _bounds[0],
        "ymin": _bounds[1],
        "xmax": _bounds[2],
        "ymax": _bounds[3],
        "xres": _res[0],
        "yres": _res[1],
        "nrows": _shape[0],
        "ncols": _shape[1],
        "has_extent": has_extent
    }

    # extract columns of interest and convert to polars
    cols = list(set([col for col in (field, by) if col]))
    df = pl.from_pandas(gdf[cols]) if cols else None

    # rusterize
    r = _rusterize(
        gdf.geometry,
        raster_info,
        fun,
        df,
        field,
        by,
        background
    )
    return DataArray.from_dict(r).rio.write_crs(gdf.crs, inplace=True)