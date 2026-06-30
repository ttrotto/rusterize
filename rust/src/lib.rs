#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod prelude;
#[doc(hidden)]
pub mod rasterize;
#[doc(hidden)]
pub mod geo {
    pub(crate) mod edges;
    pub mod raster;
}
#[doc(hidden)]
pub mod rasterization {
    pub(crate) mod burn_geometry;
    pub(crate) mod burners;
    pub(crate) mod pixel_cache;
    pub mod pixel_functions;
}
#[doc(hidden)]
pub mod encoding {
    pub mod arrays;
    pub(crate) mod writers;
}

#[doc(inline)]
pub use crate::{
    encoding::arrays::{DenseArray, SparseArray},
    error::{RusterizeError, RusterizeResult},
    geo::raster::{RasterInfo, RasterInfoBuilder},
    prelude::{NaNAware, RasterDtype, RasterizeContext},
    rasterization::pixel_functions::PixelFunction,
    rasterize::{ArrayBuilder, FieldSource, Rasterize},
};

#[cfg(feature = "polars")]
#[doc(inline)]
pub use crate::prelude::PolarsHandler;
