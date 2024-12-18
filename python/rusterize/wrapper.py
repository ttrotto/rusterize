from typing import Optional, Tuple, Union

from geopandas import GeoDataFrame
import polars as pl
import xarray as xr

# from .rusterize import _rusterize


class _RasterInfo:
    def __init__(self,
                 gdf: GeoDataFrame,
                 res: Union[Tuple[int, ...], Tuple[float, ...]]):
        """
        Contrains information to create a raster object

        Args:
        :param gdf: geopandas dataframe to rasterize
        :param res: tuple of (xres, yres) for rasterized data
        """
        self.gdf = gdf
        self.xres, self.yres = res
        self.xmin, self.ymin, self.xmax, self.ymax = self.gdf.total_bounds


class Rusterize(_RasterInfo):
    def __init__(self,
                 field: Optional[str],
                 by: Optional[str],
                 pixel_fn: str = "last",
                 background: Union[int, float] = 0):
        """
        Fast geopandas rasterization into xarray.DataArray

        Args:
        :param field: field to rasterize
        :param by: column to rasterize, assigns each unique value to a layer in the stack based on field.
        :param pixel_fn: pixel function to use, see fasterize for options
        :param background: background value in final raster

        Returns:
            Rasterized geometries into xr.DataArray
        """
        super(_RasterInfo, self).__init__()
        self.field = field
        self.by = by
        self.pixel_fn = pixel_fn
        self.background = background

        # type checks
        if not isinstance(self.gdf, GeoDataFrame):
            raise TypeError("Must pass a valid geopandas dataframe.")
        if not isinstance(self.field, (str, type(None))):
            raise TypeError("Must pass a valid string to field.")
        if not isinstance(self.by, (str, type(None))):
            raise TypeError("Must pass a valid string to by.")
        if not isinstance(self.xres, (int, float)):
            raise TypeError("Must pass a valid x resolution.")
        if not isinstance(self.yres, (int, float)):
            raise TypeError("Must pass a valid y resolution.")
        if not isinstance(self.pixel_fn, str):
            raise TypeError("Must pass a valid string to pixel_fn.")
        if not isinstance(self.background, (int, float)):
            raise TypeError("Must pass a valid background type.")

        # value check
        if by and not field:
            raise ValueError("If by is specified, field must also be specified.")
        if any((self.xres, self.yres)) <= 0 or (type(self.xres) != type(self.yres)):
            raise ValueError("Must pass valid resolution tuple of values of consistent dtype.")

    def _to_polars(self):
        """ Drop geometry and pass data as polars dataframe. Slow for large datasets. """
        return pl.from_pandas(self.gdf.drop(columns=["geometry"]))

    def process(self) -> xr.DataArray:
        pdf = self._to_polars()
        geometry = self.gpd.geometry





