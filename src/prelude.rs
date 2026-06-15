use bitflags::bitflags;
use polars::prelude::*;
use std::ops::AddAssign;

// handle polars dtypes and conversions
pub trait PolarsHandler: Literal + Send + Sync {
    type ChunkedArrayType: PolarsNumericType<Native = Self> + 'static;
    fn polars_dtype() -> DataType;
    fn into_column(self, name: &str, len: usize) -> Column;
    fn from_named_vec(name: &str, vec: &[Self]) -> Column
    where
        Self: Sized;
}

macro_rules! impl_polars_handler {
    ($($t:ty => {
        dtype: $dtype:expr,
        catype: $catype:ty
    }),* $(,)?) => {
        $(
            impl PolarsHandler for $t {
                type ChunkedArrayType = $catype;

                fn polars_dtype() -> DataType {
                    $dtype
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
    f64 => { dtype: DataType::Float64, catype: Float64Type},
    f32 => { dtype: DataType::Float32, catype: Float32Type},
    u8  => { dtype: DataType::UInt8,   catype: UInt8Type},
    i8  => { dtype: DataType::Int8,    catype: Int8Type},
    u16 => { dtype: DataType::UInt16,  catype: UInt16Type},
    i16 => { dtype: DataType::Int16,   catype: Int16Type},
    u32 => { dtype: DataType::UInt32,  catype: UInt32Type},
    i32 => { dtype: DataType::Int32,   catype: Int32Type},
    u64 => { dtype: DataType::UInt64,  catype: UInt64Type},
    i64 => { dtype: DataType::Int64,   catype: Int64Type},
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

// optional flags at runtime
bitflags! {
    #[derive(Copy, Clone)]
    pub struct OptFlags: u32 {
        // burn all pixels that are touched by the geometry
        const ALL_TOUCHED = 1;
        // same as ALL_TOUCHED but requires cache
        const ALL_TOUCHED_CACHED = 1 << 2;
        // output return type is Xarray
        const OUT_AS_XARRAY = 1 << 3;
    }
}

impl OptFlags {
    pub fn new(all_touched: bool, encoding: &str, pixel_fn: &str) -> Self {
        let mut opt_flags = OptFlags::empty();

        if all_touched {
            opt_flags.insert(OptFlags::ALL_TOUCHED);

            if pixel_fn == "sum" || pixel_fn == "count" {
                opt_flags.insert(OptFlags::ALL_TOUCHED_CACHED);
            }
        }

        if encoding == "xarray" {
            opt_flags.insert(OptFlags::OUT_AS_XARRAY);
        }

        opt_flags
    }

    pub fn with_all_touched(&self) -> bool {
        self.contains(OptFlags::ALL_TOUCHED)
    }

    pub fn requires_deduplication(&self) -> bool {
        self.contains(OptFlags::ALL_TOUCHED_CACHED)
    }

    pub fn with_xarray_output(&self) -> bool {
        self.contains(OptFlags::OUT_AS_XARRAY)
    }
}

// structures for selecting encoding type and rasterization logic
pub struct Dense;
pub struct Sparse;
