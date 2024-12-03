/*
Structure to contain information on raster data.
 */

use ndarray::{Array2, Array3};

pub struct Raster {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
    pub nrows: usize,
    pub ncols: usize,
    pub nlyr: usize,
}

impl Raster {
    pub fn new(
        xmin: f64,
        xmax: f64,
        ymin: f64,
        ymax: f64,
        xres: f64,
        yres: f64,
        nlyr: usize,
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
            nlyr,
        }
    }

    // build 2d array
    pub fn build_2d_raster(&self) -> Array2<f64> {
        Array2::<f64>::zeros((self.nrows, self.ncols))
    }

    // build 3d array
    pub fn build_3d_raster(&self) -> Array3<f64> {
        Array3::<f64>::zeros((self.nlyr, self.nrows, self.ncols))
    }
}