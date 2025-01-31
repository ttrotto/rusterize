from __future__ import annotations

from typing import Any, Dict, Optional, Tuple, Union

import polars as pl
from pandas import DataFrame
import rioxarray
from xarray import DataArray
from .rusterize import _rusterize


def rusterize(gdf: DataFrame,
              res: Union[Tuple[int, ...], Tuple[float, ...]],
              field: Optional[str] = None,
              by: Optional[str] = None,
              pixel_fn: str = "last",
              background: Optional[Union[int, float]] = None,
              ) -> Dict[str, Any]:
    """
    Fast geopandas rasterization into xarray.DataArray

    Args:
    :param gdf: geopandas dataframe to rasterize.
    :param res: tuple of (xres, yres) for rasterized data.
    :param field: field to rasterize. Default is None.
    :param by: column to rasterize, assigns each unique value to a layer in the stack based on field. Default is None.
    :param pixel_fn: pixel function to use, see fasterize for options. Default is `last`.
    :param background: background value in final raster. Default is None.

    Returns:
        Dictionary containing rasterized geometries and spatial attributes to build a xarray.DataArray.
    """
    # type checks
    if not isinstance(gdf, DataFrame):
        raise TypeError("Must pass a valid geopandas dataframe.")
    if not isinstance(field, (str, type(None))):
        raise TypeError("Must pass a valid string to field.")
    if not isinstance(by, (str, type(None))):
        raise TypeError("Must pass a valid string to by.")
    if not isinstance(res, tuple):
        raise TypeError("Must pass a valid resolution tuple (x, y).")
    if not isinstance(pixel_fn, str):
        raise TypeError("Must pass a valid string to pixel_fn. Select only of sum, first, last, min, max, count, or any.")
    if not isinstance(background, (int, float, type(None))):
        raise TypeError("Must pass a valid background type.")

    # value check
    if by and not field:
        raise ValueError("If by is specified, field must also be specified.")
    if len(res) != 2 or any((res[0], res[1])) <= 0 or not isinstance(res[0], type(res[1])):
        raise ValueError("Must pass valid resolution tuple of values of consistent dtype.")
    if not gdf.crs.is_projected:
        raise NotImplementedError("Only projected CRS are supported.")

    # RasterInfo
    bounds = gdf.total_bounds
    raster_info = {
        "xmin": bounds[0],
        "ymin": bounds[1],
        "xmax": bounds[2],
        "ymax": bounds[3],
        "xres": res[0],
        "yres": res[1],
        "nrows": 0,
        "ncols": 0
    }

    # extract columns of interest and convert to polars
    cols = list(set([col for col in (field, by) if col]))
    df = pl.from_pandas(gdf[cols]) if cols else None

    # rusterize
    r = _rusterize(
        gdf.geometry,
        raster_info,
        pixel_fn,
        df,
        field,
        by,
        background
    )
    return DataArray.from_dict(r).rio.write_crs(gdf.crs, inplace=True)