from __future__ import annotations

from typing import Optional, Tuple, Union

import polars as pl
import xarray as xr
from pandas import DataFrame
from .rusterize import _rusterize


class Rusterize:
    def __init__(self,
                 gdf: DataFrame,
                 res: Union[Tuple[int, ...], Tuple[float, ...]],
                 field: Optional[str] = None,
                 by: Optional[str] = None,
                 pixel_fn: str = "last",
                 background: Union[int, float] = 0):
        """
        Fast geopandas rasterization into xarray.DataArray

        Args:
        :param gdf: geopandas dataframe to rasterize
        :param res: tuple of (xres, yres) for rasterized data
        :param field: field to rasterize
        :param by: column to rasterize, assigns each unique value to a layer in the stack based on field.
        :param pixel_fn: pixel function to use, see fasterize for options
        :param background: background value in final raster

        Returns:
            Rasterized geometries into xr.DataArray
        """
        self.gdf = gdf
        self.res = res
        self.field = field
        self.by = by
        self.pixel_fn = pixel_fn
        self.background = background

        # type checks
        if not isinstance(self.gdf, DataFrame):
            raise TypeError("Must pass a valid geopandas dataframe.")
        if not isinstance(self.field, (str, type(None))):
            raise TypeError("Must pass a valid string to field.")
        if not isinstance(self.by, (str, type(None))):
            raise TypeError("Must pass a valid string to by.")
        if not isinstance(self.res, tuple):
            raise TypeError("Must pass a valid resolution tuple (x, y).")
        if not isinstance(self.pixel_fn, str):
            raise TypeError("Must pass a valid string to pixel_fn.")
        if not isinstance(self.background, (int, float)):
            raise TypeError("Must pass a valid background type.")

        # mapping attributes
        self.xmin, self.ymin, self.xmax, self.ymax = self.gdf.total_bounds

        # value check
        if by and not field:
            raise ValueError("If by is specified, field must also be specified.")
        if len(self.res) != 2 or any((self.res[0], self.res[1])) <= 0 or not isinstance(self.res[0], type(self.res[1])):
            raise ValueError("Must pass valid resolution tuple of values of consistent dtype.")


    def _rasinfo(self):
        """ Create dictionary containing raster info mirrored in the Rust side """
        return {
            "xmin": self.xmin,
            "ymin": self.ymin,
            "xmax": self.xmax,
            "ymax": self.ymax,
            "xres": self.res[0],
            "yres": self.res[1],
        }

    def _to_polars(self):
        """ Extracts columns of interest and convert to polars """
        cols = [col for col in (self.field, self.by) if col]
        return pl.from_pandas(self.gdf[cols]) if cols else None

    def process(self) -> xr.DataArray:
        pf = self._to_polars()
