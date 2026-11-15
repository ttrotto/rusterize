use crate::prelude::*;
use ndarray::ArrayViewMut2;
use num_traits::Num;
use std::{ops::AddAssign, str::FromStr};

/// Supported functions to apply to overlapping pixels.
///
/// `Sum` and `Count` add each pixel at most once per geometry, even where a line
/// crosses itself or where the parts of a multi-geometry meet. With integer types they wrap around on overflow
/// (e.g. 300 overlaps in a `u8` raster give 44), so pick a type wide enough for the expected totals.
#[derive(Clone, Copy)]
pub enum PixelFunction {
    /// Sum of the values of all geometries covering the pixel.
    Sum,
    /// Value of the first geometry covering the pixel.
    First,
    /// Value of the last geometry covering the pixel.
    Last,
    /// Smallest value covering the pixel.
    Min,
    /// Largest value covering the pixel.
    Max,
    /// Number of geometries covering the pixel.
    Count,
    /// 1 if any geometry covers the pixel.
    Any,
}

impl FromStr for PixelFunction {
    type Err = RusterizeError;

    fn from_str(s: &str) -> RusterizeResult<Self> {
        match s {
            "sum" => Ok(Self::Sum),
            "first" => Ok(Self::First),
            "last" => Ok(Self::Last),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "count" => Ok(Self::Count),
            "any" => Ok(Self::Any),
            _ => Err(RusterizeError::ValueError("Unknown pixel function")),
        }
    }
}

impl PixelFunction {
    /// Apply to the pixel at (y, x).
    /// Note: A `match` instead of a fn pointer so it inlines into the burn hot loops.
    #[inline(always)]
    pub(crate) fn apply<N>(self, array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
    where
        N: Num + Copy + AddAssign + PartialOrd + NaNAware,
    {
        match self {
            Self::Sum => sum_values(array, y, x, value, bg),
            Self::First => first_values(array, y, x, value, bg),
            Self::Last => last_values(array, y, x, value, bg),
            Self::Min => min_values(array, y, x, value, bg),
            Self::Max => max_values(array, y, x, value, bg),
            Self::Count => count_values(array, y, x, value, bg),
            Self::Any => any_values(array, y, x, value, bg),
        }
    }
}

fn sum_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + AddAssign + NaNAware + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() {
        array[[y, x]] = value;
    } else {
        array[[y, x]] += value;
    }
}

fn first_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + NaNAware + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() {
        array[[y, x]] = value;
    }
}

fn last_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, _bg: N)
where
    N: Num + Copy,
{
    array[[y, x]] = value;
}

fn min_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + NaNAware + PartialOrd + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() || array[[y, x]] > value {
        array[[y, x]] = value;
    }
}

fn max_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + NaNAware + PartialOrd + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() || array[[y, x]] < value {
        array[[y, x]] = value;
    }
}

fn count_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, _value: N, bg: N)
where
    N: Num + AddAssign + NaNAware + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() {
        array[[y, x]] = N::one();
    } else {
        array[[y, x]] += N::one();
    }
}

fn any_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, _value: N, _bg: N)
where
    N: Num,
{
    array[[y, x]] = N::one();
}
