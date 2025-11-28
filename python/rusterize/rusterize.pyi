from typing import List, Tuple

import numpy as np
from geopandas import GeoDataFrame
from polars import DataFrame
from xarray import DataArray, Dataset

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
    encoding: str = "xarray",
    dtype: str = "float64",
) -> DataArray | np.ndarray | SparseArray:
    """
    Fast geopandas rasterization in Rust.

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
        :param encoding: return a dense array (burned geometries onto a raster) or a sparse array in COOrdinate format (coordinates and values of the rasterized geometries). Available options are `xarray`, `numpy`, or `sparse`. The `xarray` encoding requires `xarray` and `rioxarray` to be installed. Default is `xarray`.
        :param dtype: specify the output dtype. Default is `float64`.

    Returns:
        xarray.DataArray, numpy.ndarray, or a sparse array in COO format.

    Notes:
        When any of `res`, `out_shape`, or `extent` is not provided, it is inferred from the other arguments when applicable.
        If `like` is specified, `res`, `out_shape`, and `extent` are inferred from the `like` DataArray.
        Unless `extent` is specified, a half-pixel buffer is applied to avoid missing points on the border.
        The logics dictating the final spatial properties of the rasterized geometries follow those of GDAL.

        If `field` is not in `gdf`, then a default `burn` value of 1 is rasterized.

        A `None` value for `dtype` corresponds to the default of that dtype. An illegal value for a dtype will be replaced with the default of that dtype. For example, a `background=np.nan` for `dtype="uint8"` will become `background=0`, where `0` is the default for `uint8`.
    """

class SparseArray:
    def to_xarray(self) -> DataArray: ...
    def to_frame(self) -> DataFrame: ...
