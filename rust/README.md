## rusterize

**rusterize** is an extremely fast rasterization engine for [`geo::Geometry`](https://docs.rs/geo/latest/geo/geometry/enum.Geometry.html).

Geometries can be rasterized as a `DenseArray` (a materialized raster) or a `SparseArray`, containing the band/row/col value triplets
of all lazily burned pixels. A `SparseArray` can later be materialized into a raster, therefore avoiding large memory allocations
until it's actually needed.

### Example

Build a `RasterInfo` describing the output grid, wrap it in a `RasterizeContext`, then call `rasterize` on any slice of geometries.
The target type (`DenseArray` or `SparseArray`) selects the output encoding and data type. The `PixelFunction` dictates what happens
to overlapping pixels. `FieldSource` represents the values to be burned.

```rust
use rusterize::prelude::*;
use geo::{Geometry, Point};

fn example() -> RusterizeResult<()> {
    let raster_info = RasterInfoBuilder::new()
        .extent(0.0, 0.0, 10.0, 10.0)
        .resolution(1.0, 1.0)
        .build()?;

    let geoms = vec![Geometry::Point(Point::new(5.0, 5.0)), Geometry::Point(Point::new(3.0, 3.0))];

    let ctx = RasterizeContext {
        raster_info,
        field: FieldSource::Scalar(1.0_f64),
        by: None,
        pixel_fn: PixelFunction::Last,
        background: f64::NAN,
        all_touched: false,
    };

    let raster = geoms.rasterize::<DenseArray<f64>>(ctx)?;
    Ok(())
}
```

### Feature flags

- `polars`: Adds `FieldSource::Column` for burning a [`polars`](https://docs.rs/polars/latest/polars/) column.
