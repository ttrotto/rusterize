/*
On-demand functions for geometry rasterizetion.
 */
use crate::prelude::*;
use num_traits::Num;
use numpy::ndarray::ArrayViewMut2;
use std::ops::AddAssign;

pub type PixelFn<T> = fn(&mut ArrayViewMut2<T>, usize, usize, &T, &T);

// sum values or NaN/background
fn sum_values<T>(array: &mut ArrayViewMut2<T>, y: usize, x: usize, value: &T, bg: &T)
where
    T: Num + AddAssign + NaNAware + Copy,
{
    if array[[y, x]].eq(bg) || array[[y, x]].is_nan() || value.is_nan() {
        array[[y, x]] = *value;
    } else {
        array[[y, x]] += *value;
    }
}

// set first value only if currently NaN/background
fn first_values<T>(array: &mut ArrayViewMut2<T>, y: usize, x: usize, value: &T, bg: &T)
where
    T: Num + NaNAware + Copy,
{
    if array[[y, x]].eq(bg) || array[[y, x]].is_nan() {
        array[[y, x]] = *value;
    }
}

// always set last value
fn last_values<T>(array: &mut ArrayViewMut2<T>, y: usize, x: usize, value: &T, _bg: &T)
where
    T: Num + Copy,
{
    array[[y, x]] = *value;
}

// set value if smaller than current
fn min_values<T>(array: &mut ArrayViewMut2<T>, y: usize, x: usize, value: &T, bg: &T)
where
    T: Num + NaNAware + PartialOrd + Copy,
{
    if array[[y, x]].eq(bg) || array[[y, x]].is_nan() || array[[y, x]].gt(value) {
        array[[y, x]] = *value;
    }
}

// set value if larger than current
fn max_values<T>(array: &mut ArrayViewMut2<T>, y: usize, x: usize, value: &T, bg: &T)
where
    T: Num + NaNAware + PartialOrd + Copy,
{
    if array[[y, x]].eq(bg) || array[[y, x]].is_nan() || array[[y, x]].lt(value) {
        array[[y, x]] = *value;
    }
}

// count values at index
fn count_values<T>(array: &mut ArrayViewMut2<T>, y: usize, x: usize, _value: &T, bg: &T)
where
    T: Num + AddAssign + NaNAware + Copy,
{
    if array[[y, x]].eq(bg) || array[[y, x]].is_nan() {
        array[[y, x]] = T::one();
    } else {
        array[[y, x]] += T::one();
    }
}

// mark value presence
fn any_values<T>(array: &mut ArrayViewMut2<T>, y: usize, x: usize, _value: &T, _bg: &T)
where
    T: Num,
{
    array[[y, x]] = T::one();
}

// function call
pub fn set_pixel_function<T>(fstr: &str) -> PixelFn<T>
where
    T: Num + Copy + PixelOps,
{
    match fstr {
        "sum" => sum_values,
        "first" => first_values,
        "last" => last_values,
        "min" => min_values,
        "max" => max_values,
        "count" => count_values,
        "any" => any_values,
        _ => panic!(
            "'fun' has an invalid value: {fstr}. One of sum, first, last, min, max, count, or any",
        ),
    }
}
