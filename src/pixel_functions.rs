/*
On-demand return functions for overlapping polygons
 */

use ndarray::Array2;

// declare a new type
type PixelFn = fn(&mut Array2<f64>, usize, usize, &f64);

// add value or NA
fn sum_or_na(array: &mut Array2<f64>,
             x: usize,
             y: usize,
             value: &f64) -> () {
    array[[y, x]] = *value
}