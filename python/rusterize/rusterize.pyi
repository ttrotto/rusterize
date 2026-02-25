import numpy as np

from ._dependencies import geopandas as gpd
from ._dependencies import polars as pl
from ._dependencies import xarray as xr

def rusterize(
    data: gpd.GeoDataFrame | pl.DataFrame | list | np.ndarray,
    like: xr.DataArray | xr.Dataset | None = None,
    res: tuple | list | None = None,
    out_shape: tuple | list | None = None,
    extent: tuple | list | None = None,
    field: str | None = None,
    by: str | None = None,
    burn: int | float | None = None,
    fun: str = "last",
    background: int | float | None = np.nan,
    encoding: str = "xarray",
    all_touched: bool = False,
    tap: bool = False,
    dtype: str = "float64",
) -> xr.DataArray | np.ndarray | SparseArray:
    """
    Fast geometry rasterization in Rust.

    Parameters
    ----------
    data : geopandas.GeoDataFrame, polars.DataFrame, list, numpy.ndarray
      Input data to rasterize.
      - If polars.DataFrame, it must be have a "geometry" column with geometries stored in WKB or WKT format.
      - If list or numpy.ndarray, geometries must be in WKT, WKB, or shapely formats (EPSG is not inferred and defaults to None).
    like : xarray.DataArray or xarray.Dataset (default: None)
      Template array used as a spatial blueprint (resolution, shape, extent). Mutually exclusive with `res`, `out_shape`, and `extent`. Requires xarray and rioxarray.
    res : tuple or list (default: None)
      Pixel resolution defined as (xres, yres).
    out_shape : tuple or list (default: None)
      Output raster dimensions defined as (nrows, ncols).
    extent : `tuple` or `list` (default: None)
      Spatial bounding box defined as `(xmin, ymin, xmax, ymax)`.
    field : `str` (default: None)
      Column name to use for pixel values. Mutually exclusive with `burn`. Not considered when input is list or numpy.ndarray.
    by : `str` (default: None)
      Column used for grouping. Each group is rasterized into a distinct band in the output. Not considered when input is list or numpy.ndarray.
    burn : `int` or `float` (default: None)
      A static value to apply to all geometries. Mutually exclusive with `field`.
    fun : `str` (default: "last")
      Pixel function to use when burning geometries. Available options: `sum`, `first`, `last`, `min`, `max`, `count`, or `any`.
    background : `int` or `float` (default: numpy.nan)
      Value assigned to pixels not covered by any geometry.
    encoding : `str` (default: "xarray")
      The format of the returned object: `"xarray"`, `"numpy"`, or `"sparse"`.
    all_touched : `bool` (default: False)
      If True, every pixel touched by a geometry is burned.
    tap : `bool` (default: False)
      Target Aligned Pixels: aligns the extent to the pixel resolution.
    dtype : `str` (default: "float64")
      Output data type (e.g., `uint8`, `int32`, `float32`).

    Returns
    -------
        xarray.DataArray, numpy.ndarray, or a sparse array in COO format.

    Notes
    -----
        If `encoding` is "numpy" or input is list or numpy.ndarray, the return array is without any spatial reference.

        When any of `res`, `out_shape`, or `extent` is not provided, it is inferred from the other arguments when applicable.
        If `like` is specified, `res`, `out_shape`, and `extent` are inferred from the `like` DataArray or Dataset.
        Unless `extent` is specified, a half-pixel buffer is applied to avoid missing points on the border.
        The logics dictating the final spatial properties of the rasterized geometries follow those of GDAL.

        If `field` is not in `data`, then a default `burn` value of 1 is rasterized.

        A `None` value for `dtype` corresponds to the default of that dtype. An illegal value for a dtype will be replaced with the default of that dtype. For example, a `background=np.nan` for `dtype="uint8"` will become `background=0`, where `0` is the default for `uint8`.
    """

class SparseArray:
    def to_xarray(self) -> xr.DataArray: ...
    def to_numpy(self) -> np.ndarray: ...
    def to_frame(self) -> pl.DataFrame: ...
