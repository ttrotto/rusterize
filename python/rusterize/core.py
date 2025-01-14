from __future__ import annotations

from typing import Optional, Tuple, Union

import polars as pl
from xarray.core.dataarray import DataArray
from pandas import DataFrame
from .rusterize import _rusterize


class _RasterInfo:
    def __init__(self,
                 bounds: Tuple[float,...],
                 res: Union[Tuple[int, ...], Tuple[float, ...]]):
        """ Mirrors RasterInfo class in Rust """
        self.xmin, self.ymin, self.xmax, self.ymax = bounds
        self.xres, self.yres = res
        self.nrows, self.ncols = 0, 0


def rusterize(gdf: DataFrame,
              res: Union[Tuple[int, ...], Tuple[float, ...]],
              field: Optional[str] = None,
              by: Optional[str] = None,
              pixel_fn: str = "last",
              background: Union[int, float] = None,
              threads: int = 4
              ) -> DataArray:
    """
    Fast geopandas rasterization into xarray.DataArray

    Args:
    :param gdf: geopandas dataframe to rasterize.
    :param res: tuple of (xres, yres) for rasterized data.
    :param field: field to rasterize. Default is None.
    :param by: column to rasterize, assigns each unique value to a layer in the stack based on field. Default is None.
    :param pixel_fn: pixel function to use, see fasterize for options. Default is `last`.
    :param background: background value in final raster. Default is None.
    :param threads: number of threads to use when `by` is specified. Set to -1 to use all threads. Default is 4.

    Returns:
        Rasterized geometries into xr.DataArray
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
    if not isinstance(background, (int, float)):
        raise TypeError("Must pass a valid background type.")
    if not isinstance(threads, int):
        raise TypeError("Must pass a valid thread number")

    # value check
    if by and not field:
        raise ValueError("If by is specified, field must also be specified.")
    if len(res) != 2 or any((res[0], res[1])) <= 0 or not isinstance(res[0], type(res[1])):
        raise ValueError("Must pass valid resolution tuple of values of consistent dtype.")
    if not gdf.crs.is_projected:
        raise NotImplementedError("Only projected CRS are supported.")

    # RasterInfo
    raster_info = _RasterInfo(gdf.total_bounds, res)

    # extract columns of interest and convert to polars
    cols = list(set([col for col in (field, by) if col]))
    df = pl.from_pandas(gdf[cols]) if cols else None

    # rusterize
    return _rusterize(
        gdf.geometry,
        raster_info,
        pixel_fn,
        threads,
        df,
        field,
        by,
        background
    )
