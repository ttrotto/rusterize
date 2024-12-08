/*
Structure to contain information on raster data.
 */

use pyo3::prelude::*;
use numpy::ndarray::Array3;

#[derive(FromPyObject)]
pub struct Raster {
    pub nrows: usize,
    pub ncols: usize,
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
}

impl Raster {
    // map Raster with PyAny information
    pub fn from(pyinfo: &PyAny) -> Self {
        let raster_info: Raster = pyinfo.extract().expect("Failed to extract raster information");
        let nrows = ((raster_info.ymax - raster_info.ymin) / raster_info.yres).ceil() as usize;
        let ncols = ((raster_info.xmax - raster_info.xmin) / raster_info.xres).ceil() as usize;
        Self {
            nrows,
            ncols,
            ..raster_info
        }
    }

    // build raster
    pub fn build_raster(&self, nlyr: usize) -> Array3<f64> {
        Array3::<f64>::zeros((nlyr, self.nrows, self.ncols))
    }
}