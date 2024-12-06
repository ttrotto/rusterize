/*
Structure to contain information on raster data.
 */

use numpy::ndarray::{Array2, Array3};

pub struct Raster {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
    pub nrows: usize,
    pub ncols: usize,
}

impl Raster {
    pub fn new(
        xmin: f64,
        xmax: f64,
        ymin: f64,
        ymax: f64,
        xres: f64,
        yres: f64,
    ) -> Self {
        let nrows = ((ymax - ymin) / yres).ceil() as usize;
        let ncols = ((xmax - xmin) / xres).ceil() as usize;
        Self {
            xmin,
            xmax,
            ymin,
            ymax,
            xres,
            yres,
            nrows,
            ncols,
        }
    }

    // build raster
    pub fn build_raster(&self, nlyr: usize) -> Array3<f64> {
        Array3::<f64>::zeros((nlyr, self.nrows, self.ncols))
    }
}