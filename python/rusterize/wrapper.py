from typing import Optional, Tuple, Union

from geopandas import GeoDataFrame
import polars as pl
import xarray as xr

SUPPORTED_GEOM = {"Polygon", "Multipolygon"}


class Rusterize:
    """ Fast geopandas rusterization into xarray.DataArray """
    def __init__(self,
                 gdf: GeoDataFrame,
                 field: Optional[str],
                 by: Optional[str],
                 res: Union[Tuple[int, ...], Tuple[float, ...]],
                 pixel_fn: str = "last",
                 background: Union[int, float] = 0):
        """
        Fast geopandas rusterization into xarray.DataArray

        Args:
            gdf: geopandas dataframe to rasterize
            field: field to rasterize
            by: column to rasterize, assigns each unique value to a layer in the stack based on field.
            res: tuple of (xres, yres) for rasterized data
            pixel_fn: pixel function to use, see fasterize for options
            background: background value in final raster

        Returns:
            Rasterized value into xr.DataArray
        """
        self.gdf = gdf
        self.field = field
        self.by = by
        self.res = res
        self.pixel_fn = pixel_fn
        self.background = background

        # type checks
        if not isinstance(self.gdf, GeoDataFrame):
            raise TypeError("Must pass a valid geopandas dataframe.")
        if not isinstance(self.field, (str, type(None))):
            raise TypeError("Must pass a valid string to field.")
        if not isinstance(self.by, (str, type(None))):
            raise TypeError("Must pass a valid string to by.")
        if not isinstance(self.res, (Tuple[int, ...], Tuple[float, ...])):
            raise TypeError("Must pass a valid resolution tuple.")
        if not isinstance(self.pixel_fn, str):
            raise TypeError("Must pass a valid string to pixel_fn.")
        if not isinstance(self.background, (int, float)):
            raise TypeError("Must pass a valid background type.")

        # value check
        if by and not field:
            raise ValueError("If by is specified, field must also be specified.")
        if len(self.res) < 2 or any(self.res) <= 0 or any(not isinstance(x, (int, float)) for x in self.res):
            raise ValueError("Must pass valid resolution tuple values.")
        if self.pixel_fn not in ["sum", "first", "last", "min", "max", "count", "any"]:
            raise ValueError("pixel_fn must be one of sum, first, last, min, max, count, or any.")

        # geom type check
        geom_types = set(self.gdf.geom_type)
        if geom_types > SUPPORTED_GEOM or len(geom_types & SUPPORTED_GEOM) != 1:
            raise NotImplementedError("Only Polygon and Multipolygon geometry types are supported.")

    def __call__(self):
        return self._rusterize()

    def _to_polars(self):
        return pl.from_pandas(self.gdf)

    # def _pass_bounds(self) -> List:
    #     """ Round down and up total bounds """
    #     b = self.gdf.total_bounds
    #     b[0], b[1] = floor(b[0]), floor(b[1])  # xmin, ymin
    #     b[2], b[3] = ceil(b[2]), ceil(b[3])  # xmax, ymax
    #     return b

    def _rusterize(self) -> xr.DataArray:
        pdf = self._to_polars()


