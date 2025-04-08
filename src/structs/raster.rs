/*
Structure to contain information on raster data.
 */

use geo::Rect;
use numpy::{
    IntoPyArray, PyArray1,
    ndarray::{Array, Array3},
};
use pyo3::prelude::*;

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
pub struct RasterInfo {
    pub nrows: usize,
    pub ncols: usize,
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
    pub has_extent: bool,
}

impl RasterInfo {
    pub fn from(pyinfo: &Bound<PyAny>) -> Self {
        // map PyAny to RasterInfo
        let raster_info: RasterInfo = pyinfo
            .extract()
            .expect("Wrong mapping passed to RasterInfo struct");

        raster_info
    }

    pub fn update_dims(&mut self) {
        // extend bounds by half pixel to avoid missing points on the border
        if !self.has_extent && self.xres != 0.0 {
            self.xmin -= self.xres / 2.0;
            self.xmax += self.xres / 2.0;
            self.ymin -= self.yres / 2.0;
            self.ymax += self.yres / 2.0;
        }

        // calculate resolution
        if self.xres == 0.0 {
            self.resolution();
        }

        // calculate shape
        if self.nrows == 0 {
            self.shape();
        }
    }

    fn shape(&mut self) {
        self.nrows = ((self.ymax - self.ymin) / self.yres).round() as usize;
        self.ncols = ((self.xmax - self.xmin) / self.xres).round() as usize;
    }

    fn resolution(&mut self) {
        self.xres = (self.xmax - self.xmin) / self.ncols as f64;
        self.yres = (self.ymax - self.ymin) / self.nrows as f64;
    }

    pub fn update_bounds(&mut self, rect: Rect) {
        // update bounding box
        self.xmin = rect.min().x;
        self.xmax = rect.max().x;
        self.ymin = rect.min().y;
        self.ymax = rect.max().y;
    }

    pub fn build_raster(&self, bands: usize, background: f64) -> Array3<f64> {
        Array3::from_elem((bands, self.nrows, self.ncols), background)
    }

    // construct coordinates for xarray (start from pixel's center)
    pub fn make_coordinates<'py>(
        &self,
        py: Python<'py>,
    ) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>) {
        let y_coords = Array::range(
            self.ymax - self.yres / 2.0,
            self.ymax - self.nrows as f64 * self.yres,
            -self.yres,
        )
        .into_pyarray(py);
        let x_coords = Array::range(
            self.xmin + self.xres / 2.0,
            self.xmin + self.ncols as f64 * self.xres,
            self.xres,
        )
        .into_pyarray(py);
        (y_coords, x_coords)
    }
}
