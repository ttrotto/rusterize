use crate::rasterization::pixel_functions::PixelFn;
use num_traits::Num;
use std::ops::AddAssign;

pub use crate::{
    encoding::arrays::{DenseArray, SparseArray},
    error::{RusterizeError, RusterizeResult},
    geo::raster::{RasterInfo, RasterInfoBuilder},
    rasterization::pixel_functions::PixelFunction,
    rasterize::{ArrayBuilder, FieldSource, Rasterize},
};

/// Trait to handle NaN check for dtypes that don't have it.
pub trait NaNAware {
    fn is_nan(&self) -> bool;
}

impl NaNAware for f32 {
    fn is_nan(&self) -> bool {
        f32::is_nan(*self)
    }
}

impl NaNAware for f64 {
    fn is_nan(&self) -> bool {
        f64::is_nan(*self)
    }
}

macro_rules! impl_maybe_nan_for_int {
    ($($t:ty),*) => {
        $(impl NaNAware for $t {
            fn is_nan(&self) -> bool {
                false
            }
        })*
    };
}

impl_maybe_nan_for_int!(u8, u16, u32, u64, i8, i16, i32, i64);

/// Handle polars dtypes and conversions.
#[cfg(feature = "polars")]
pub trait PolarsHandler: polars::prelude::Literal + Send + Sync {
    type ChunkedArrayType: polars::prelude::PolarsNumericType<Native = Self> + 'static;
    fn polars_dtype() -> polars::prelude::DataType;
    fn from_named_vec(name: &str, vec: &[Self]) -> polars::prelude::Column
    where
        Self: Sized;
}

#[cfg(feature = "polars")]
macro_rules! impl_polars_handler {
    ($($t:ty => { dtype: $dtype:expr, catype: $catype:ty }),* $(,)?) => {
        $(
            impl PolarsHandler for $t {
                type ChunkedArrayType = $catype;
                fn polars_dtype() -> polars::prelude::DataType { $dtype }
                fn from_named_vec(name: &str, vec: &[Self]) -> polars::prelude::Column {
                    polars::prelude::Column::new(name.into(), vec)
                }
            }
        )*
    };
}

#[cfg(feature = "polars")]
impl_polars_handler! {
    f64 => { dtype: polars::prelude::DataType::Float64, catype: polars::prelude::Float64Type },
    f32 => { dtype: polars::prelude::DataType::Float32, catype: polars::prelude::Float32Type },
    u8  => { dtype: polars::prelude::DataType::UInt8,   catype: polars::prelude::UInt8Type },
    i8  => { dtype: polars::prelude::DataType::Int8,    catype: polars::prelude::Int8Type },
    u16 => { dtype: polars::prelude::DataType::UInt16,  catype: polars::prelude::UInt16Type },
    i16 => { dtype: polars::prelude::DataType::Int16,   catype: polars::prelude::Int16Type },
    u32 => { dtype: polars::prelude::DataType::UInt32,  catype: polars::prelude::UInt32Type },
    i32 => { dtype: polars::prelude::DataType::Int32,   catype: polars::prelude::Int32Type },
    u64 => { dtype: polars::prelude::DataType::UInt64,  catype: polars::prelude::UInt64Type },
    i64 => { dtype: polars::prelude::DataType::Int64,   catype: polars::prelude::Int64Type },
}

/// Bound rasterization to a dtype.
#[cfg(feature = "polars")]
pub trait RasterDtype: Num + Copy + AddAssign + PartialOrd + NaNAware + PolarsHandler {}
#[cfg(feature = "polars")]
impl<N: Num + Copy + AddAssign + PartialOrd + NaNAware + PolarsHandler> RasterDtype for N {}
#[cfg(not(feature = "polars"))]
pub trait RasterDtype: Num + Copy + AddAssign + PartialOrd + NaNAware + Send + Sync {}
#[cfg(not(feature = "polars"))]
impl<N: Num + Copy + AddAssign + PartialOrd + NaNAware + Send + Sync> RasterDtype for N {}

/// Spatial + value context handed to the rasterization engine.
#[derive(Clone)]
pub struct RasterizeContext<'a, N> {
    /// The spatial information of the final raster.
    pub raster_info: RasterInfo,
    /// The values to burn.
    pub field: FieldSource<'a, N>,
    /// Specify the grouping of the geometries into multiple bands in the final raster. None is no grouping.
    /// For this to work, `by` has to have the same length of the geometries.
    pub by: Option<&'a [String]>,
    /// Describes what happens to overlapping pixels.
    pub pixel_fn: PixelFunction,
    pub background: N,
    /// Flags whether all pixels touching the geometry should be burned.
    pub all_touched: bool,
}

impl<'a, N> RasterizeContext<'a, N> {
    pub(crate) fn pixel_fn(&self) -> PixelFn<N>
    where
        N: Num + Copy + AddAssign + PartialOrd + NaNAware,
    {
        self.pixel_fn.to_function()
    }

    pub(crate) fn requires_dedup(&self) -> bool {
        self.all_touched && matches!(self.pixel_fn, PixelFunction::Sum | PixelFunction::Count)
    }
}
