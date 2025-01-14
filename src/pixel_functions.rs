/*
On-demand functions for polygon rasterizetion.
 */

use numpy::ndarray::ArrayViewMut2;

// declare a new type
pub type PixelFn = fn(&mut ArrayViewMut2<f64>, usize, usize, &f64);

// sum values or NA
#[inline]
fn sum_values(array: &mut ArrayViewMut2<f64>, y: usize, x: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() || value.is_nan() {
        array[[y, x]] = *value;
    } else {
        array[[y, x]] += *value;
    }
}

// set first value only if currently NA
#[inline]
fn first_values(array: &mut ArrayViewMut2<f64>, y: usize, x: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() {
        array[[y, x]] = *value;
    }
}

// always set last value
#[inline]
fn last_values(array: &mut ArrayViewMut2<f64>, y: usize, x: usize, value: &f64) -> () {
    array[[y, x]] = *value;
}

// set value if smaller than current
#[inline]
fn min_values(array: &mut ArrayViewMut2<f64>, y: usize, x: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() || array[[y, x]] > *value {
        array[[y, x]] = *value;
    }
}

// set value if larger than current
#[inline]
fn max_values(array: &mut ArrayViewMut2<f64>, y: usize, x: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() || array[[y, x]] < *value {
        array[[y, x]] = *value;
    }
}

// count values at index
#[inline]
fn count_values(array: &mut ArrayViewMut2<f64>, y: usize, x: usize, _value: &f64) -> () {
    if array[[y, x]].is_nan() {
        array[[y, x]] = 1.0;
    } else {
        array[[y, x]] += 1.0;
    }
}

// mark value presence
#[inline]
fn any_values(array: &mut ArrayViewMut2<f64>, y: usize, x: usize, _value: &f64) -> () {
    array[[y, x]] = 1.0;
}

// function call
pub fn set_pixel_function(fstr: &str) -> PixelFn {
    match fstr {
        "sum" => sum_values,
        "first" => first_values,
        "last" => last_values,
        "min" => min_values,
        "max" => max_values,
        "count" => count_values,
        "any" => any_values,
        _ => panic!("'fun' has an invalid value: {}. Select only of sum, first, last, min, max, count, or any", fstr),
    }
}
