use crate::prelude::*;
use ndarray::ArrayViewMut2;
use num_traits::Num;
use std::{ops::AddAssign, str::FromStr};

/// Supported functions to apply to overlapping pixels.
#[derive(Clone)]
pub enum PixelFunction {
    Sum,
    First,
    Last,
    Min,
    Max,
    Count,
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
    pub(crate) fn to_function<N>(&self) -> PixelFn<N>
    where
        N: Num + Copy + AddAssign + PartialOrd + NaNAware,
    {
        match self {
            Self::Sum => sum_values,
            Self::First => first_values,
            Self::Last => last_values,
            Self::Min => min_values,
            Self::Max => max_values,
            Self::Count => count_values,
            Self::Any => any_values,
        }
    }
}

/// On-demand function for overlapping pixels.
pub(crate) type PixelFn<N> = fn(&mut ArrayViewMut2<N>, usize, usize, N, N);

/// Sum values or NaN/background.
fn sum_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + AddAssign + NaNAware + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() || value.is_nan() {
        array[[y, x]] = value;
    } else {
        array[[y, x]] += value;
    }
}

/// Set first value only if currently NaN/background.
fn first_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + NaNAware + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() {
        array[[y, x]] = value;
    }
}

/// Always set last value.
fn last_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, _bg: N)
where
    N: Num + Copy,
{
    array[[y, x]] = value;
}

/// Set value if smaller than current.
fn min_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + NaNAware + PartialOrd + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() || array[[y, x]] > value {
        array[[y, x]] = value;
    }
}

/// Set value if larger than current.
fn max_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, value: N, bg: N)
where
    N: Num + NaNAware + PartialOrd + Copy,
{
    if array[[y, x]] == bg || array[[y, x]].is_nan() || array[[y, x]] < value {
        array[[y, x]] = value;
    }
}

/// Count values at position.
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

/// Mark presence.
fn any_values<N>(array: &mut ArrayViewMut2<N>, y: usize, x: usize, _value: N, _bg: N)
where
    N: Num,
{
    array[[y, x]] = N::one();
}
