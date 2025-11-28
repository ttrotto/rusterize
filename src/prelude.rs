/* Traits to handle dtype runtime polymorphism */

use polars::prelude::*;
use std::ops::AddAssign;

// handle polars dtypes and conversions
pub trait PolarsHandler: Literal + Send + Sync {
    fn polars_dtype() -> DataType;
    fn from_anyvalue(val: AnyValue) -> Option<Self>
    where
        Self: Sized;
    fn into_column(self, name: &str, len: usize) -> Column;
    fn from_named_vec(name: &str, vec: &[Self]) -> Column
    where
        Self: Sized;
}

macro_rules! impl_polars_handler {
    ($($t:ty => {
        dtype: $dtype:expr,
        anyvalue: $anyvalue:pat => $extract:expr,
    }),* $(,)?) => {
        $(
            impl PolarsHandler for $t {
                fn polars_dtype() -> DataType {
                    $dtype
                }

                fn from_anyvalue(val: AnyValue) -> Option<Self> {
                    match val {
                        $anyvalue => Some($extract),
                        _ => None,
                    }
                }

                fn into_column(self, name: &str, len: usize) -> Column {
                    Column::new(name.into(), vec![self; len])
                }

                fn from_named_vec(name: &str, vec: &[Self]) -> Column {
                    Column::new(name.into(), vec)
                }
            }
        )*
    };
}

impl_polars_handler! {
    f64 => {
        dtype: DataType::Float64,
        anyvalue: AnyValue::Float64(v) => v,
    },
    f32 => {
        dtype: DataType::Float32,
        anyvalue: AnyValue::Float32(v) => v,
    },
    u8 => {
        dtype: DataType::UInt8,
        anyvalue: AnyValue::UInt8(v) => v,
    },
    i8 => {
        dtype: DataType::Int8,
        anyvalue: AnyValue::Int8(v) => v,
    },
    u16 => {
        dtype: DataType::UInt16,
        anyvalue: AnyValue::UInt16(v) => v,
    },
    i16 => {
        dtype: DataType::Int16,
        anyvalue: AnyValue::Int16(v) => v,
    },
    u32 => {
        dtype: DataType::UInt32,
        anyvalue: AnyValue::UInt32(v) => v,
    },
    i32 => {
        dtype: DataType::Int32,
        anyvalue: AnyValue::Int32(v) => v,
    },
    u64 => {
        dtype: DataType::UInt64,
        anyvalue: AnyValue::UInt64(v) => v,
    },
    i64 => {
        dtype: DataType::Int64,
        anyvalue: AnyValue::Int64(v) => v,
    },
}

// handle NaN check for dtype that don't have it
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

// super trait to group all pixel operations
pub trait PixelOps: AddAssign + PartialOrd + NaNAware + Sized {}
impl<T: AddAssign + PartialOrd + NaNAware> PixelOps for T {}

// structures for selecting encoding type and rasterization logic
pub struct Dense;
pub struct Sparse;

pub enum OutputType {
    Numpy,
    Xarray,
}

impl OutputType {
    pub fn new(encoding: &str) -> Self {
        match encoding {
            "numpy" => Self::Numpy,
            "xarray" => Self::Xarray,
            _ => Self::Numpy, // fallback placeholder
        }
    }
}
