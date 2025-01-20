/*
Structure to contain information on raster data.
 */

use dict_derive::FromPyObject;
use numpy::ndarray::Array3;
use pyo3::prelude::*;

#[derive(FromPyObject)]
pub struct RasterInfo {
    pub nrows: usize,
    pub ncols: usize,
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
}

impl RasterInfo {
    // map PyAny to RasterInfo
    pub fn from(pyinfo: &Bound<PyAny>) -> Self {
        let raster_info: RasterInfo = pyinfo
            .extract()
            .expect("Wrong mapping passed to Raster object");
        let nrows = ((raster_info.ymax - raster_info.ymin) / raster_info.yres).round() as usize;
        let ncols = ((raster_info.xmax - raster_info.xmin) / raster_info.xres).round() as usize;
        Self {
            nrows,
            ncols,
            ..raster_info
        }
    }

    // build 3D raster
    pub fn build_raster(&self, bands: usize, background: f64) -> Array3<f64> {
        Array3::from_elem((bands, self.nrows, self.ncols), background)
    }
}
