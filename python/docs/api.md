# API reference

::: rusterize.rusterize

## SparseArray

Returned when `encoding="sparse"`. A COO-format sparse array with three converters:

- `to_xarray()` &rarr; `xarray.DataArray`
- `to_numpy()` &rarr; `numpy.ndarray`
- `to_frame()` &rarr; `polars.DataFrame`
