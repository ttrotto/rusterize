/*
On-demand functions for polygon rasterizetion.
 */

use numpy::ndarray::ArrayViewMut2;

// declare a new type
pub type PixelFn = fn(&mut ArrayViewMut2<f64>, usize, usize, &f64);

// sum values or NA
fn sum_values(array: &mut ArrayViewMut2<f64>, x: usize, y: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() || value.is_nan() {
        array[[y, x]] = *value;
    } else {
        array[[y, x]] = array[[y, x]] + *value;
    }
}

// set first value only if currently NA
fn first_values(array: &mut ArrayViewMut2<f64>, x: usize, y: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() {
        array[[y, x]] = *value;
    }
}

// always set last value
fn last_values(array: &mut ArrayViewMut2<f64>, x: usize, y: usize, value: &f64) -> () {
    array[[y, x]] = *value;
}

// set value if smaller than current
fn min_values(array: &mut ArrayViewMut2<f64>, x: usize, y: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() || array[[y, x]] > *value {
        array[[y, x]] = *value;
    }
}

// set value if larger than current
fn max_values(array: &mut ArrayViewMut2<f64>, x: usize, y: usize, value: &f64) -> () {
    if array[[y, x]].is_nan() || array[[y, x]] < *value {
        array[[y, x]] = *value;
    }
}

// count values at index
fn count_values(array: &mut ArrayViewMut2<f64>, x: usize, y: usize, _value: &f64) -> () {
    if array[[y, x]].is_nan() {
        array[[y, x]] = 1.0;
    } else {
        array[[y, x]] += 1.0;
    }
}

// mark value presence
fn any_values(array: &mut ArrayViewMut2<f64>, x: usize, y: usize, _value: &f64) -> () {
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
