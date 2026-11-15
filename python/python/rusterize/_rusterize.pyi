from typing import Any

import numpy as np

from ._dependencies import polars as pl
from ._dependencies import xarray as xr

def _rusterize(
    geometry: Any,
    raw_raster_info: dict[str, Any],
    pypixel_fn: str,
    pycolumn: pl.Series | None = None,
    pyby_codes: np.ndarray | None = None,
    pyby_labels: list[str] | None = None,
    pyburn: Any | None = None,
    pybackground: Any | None = None,
    pytouched: bool = False,
    pyencoding: str = "xarray",
    pydtype: str = "float64",
) -> xr.DataArray | np.ndarray | SparseArray: ...

class SparseArray:
    def to_xarray(self) -> xr.DataArray: ...
    def to_numpy(self) -> np.ndarray: ...
    def to_frame(self) -> pl.DataFrame: ...
