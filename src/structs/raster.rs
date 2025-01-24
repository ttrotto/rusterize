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

    fn calculate_dimensions(&self) -> (usize, usize) {
        let nrows = ((self.ymax - self.ymin) / self.yres).round() as usize;
        let ncols = ((self.xmax - self.xmin) / self.xres).round() as usize;
        (nrows, ncols)
    }

    pub fn update_bounds(&mut self, rect: Rect) {
        // update bounding box
        self.xmin = rect.min().x;
        self.xmax = rect.max().x;
        self.ymin = rect.min().y;
        self.ymax = rect.max().y;
        
        // ...and dimensions
        let (nrows, ncols) = self.calculate_dimensions();
        self.nrows = nrows;
        self.ncols = ncols;
    }
    
    pub fn build_raster(&self, bands: usize, background: f64) -> Array3<f64> {
        Array3::from_elem((bands, self.nrows, self.ncols), background)
    }
}
