# Python API reference

::: rusterize.rusterize

## SparseArray

Returned when `encoding="sparse"`. Currently, this internal structure holds the triplets of (band, row, col) values
for each lazily burned pixel, thus avoiding to materialize a full array in memory. This is advantageous when you have
large empty portions or want to save memory until you actually need it.

A `SparseArray` has the following converters:

- `to_xarray()` &rarr; `xarray.DataArray`
- `to_numpy()` &rarr; `numpy.ndarray`
- `to_frame()` &rarr; `polars.DataFrame`
