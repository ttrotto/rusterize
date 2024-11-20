from typing import List, Optional, Union

from math import floor, ceil
import geopandas as gpd
import xarray as xr


class Rusterize:
    """ Fast geopandas rusterization into xarray.DataArray """
    def __init__(self,
                 gdf: gpd.GeoDataFrame,
                 field: Optional[str],
                 by: Optional[str],
                 res: Union[int, floor],
                 pixel_fn: str = "max"):
        """
        Fast geopandas rusterization into xarray.DataArray

        Args:
            gdf: geopandas dataframe to rasterize
            pixel_fn: pixel function to use, see fasterize manual for options
            field: field to rasterize
            by: column to rasterize, assigns each unique value to a layer in the stack.
                Mutually exclusive with field.

        Returns:
            Rasterized value into xr.DataArray
        """
        self.gdf = gdf
        self.field = field
        self.by = by
        self.res = res
        self.pixel_fn = pixel_fn

        # type checks
        if not isinstance(self.gdf, gpd.GeoDataFrame):
            raise TypeError("Must pass a valid geopandas dataframe.")
        if not isinstance(self.field, (str, type(None))):
            raise TypeError("Must pass a valid string to field.")
        if not isinstance(self.by, (str, type(None))):
            raise TypeError("Must pass a valid string to by.")
        if not isinstance(self.res, (int, float)):
            raise TypeError("Must pass a valid resolution type.")
        if not isinstance(self.pixel_fn, str):
            raise TypeError("Must pass a valid string to pixel_fn.")

        # value check
        if self.res < 0:
            raise ValueError("Must pass a valid resolution value.")
        if self.pixel_fn not in ["sum", "first", "last", "min", "max", "count", "any"]:
            raise ValueError("pixel_fn must be one of sum, first, last, min, max, count, or any.")

        # geom type check
        geom_types = set(self.gdf.geom_type)
        if len(geom_types) > 1 or "Polygon" not in geom_types:
            raise NotImplementedError("Only Polygon geometry type is supported.")

    def __call__(self):
        if self.by:
            return self.rusterize_by()
        else:
            return self.rusterize()

    def _pass_geom(self) -> List:
        """
        Geometries passthrough: distill geometry coordinates as list.
        Needed because poor integration between geopandas and Rust.
        """
        return self.gdf.geometry.apply(lambda geom: list(geom.exterior.coords))

    def _pass_bounds(self) -> List:
        """ Round down and up total bounds """
        b = self.gdf.total_bounds
        b[0], b[1] = floor(b[0]), floor(b[1])  # xmin, ymin
        b[2], b[3] = ceil(b[2]), ceil(b[3])  # xmax, ymax
        return b

    def rusterize(self) -> xr.DataArray:
        """ Rasterize field """
        crds = self._pass_geom()

    def rusterize_by(self) -> xr.DataArray:
        """ Rasterize by into stack """
        pass
