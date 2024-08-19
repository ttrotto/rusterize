/*
Structure to contain information on raster data.
PyO3 is incompatible with generic parameters, so will write a macro.
 */

use std::error::Error;
use pyo3::prelude::*;
use ndarray::{Array2, Array3};

#[pyclass]
pub struct Raster {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
    pub nlyr: usize,
    pub dtype: String,
}

#[pymethods]
impl Raster {
    #[new]
    pub fn new(xmin: f64,
               xmax: f64,
               ymin: f64,
               ymax: f64,
               xres: f64,
               yres: f64,
               nlyr: usize,
               dtype: String) -> Self {
        Self {
            xmin,
            xmax,
            ymin,
            ymax,
            xres,
            yres,
            nlyr,
            dtype,
        }
    }
}

// enumerate possible ndarrays
pub enum NDArray {
    Ax2(Array2<f64>),
    Ax3(Array3<f64>),
}

// build ndarray
pub fn build_ndarray(raster: &Raster) -> Result<NDArray, Box<dyn Error>> {
    // get array dimension
    let shape_y =  (raster.ymax - raster.ymin).ceil() as usize;
    let shape_x = (raster.xmax - raster.xmin).ceil() as usize;
    // make
    match raster.nlyr {
        1 => Ok(NDArray::Ax2(Array2::<f64>::zeros((shape_y, shape_x)))),
        _ => Ok(NDArray::Ax3(Array3::<f64>::zeros((raster.nlyr, shape_y, shape_x)))),
    }
}