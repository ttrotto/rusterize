/* Structure to contain information on raster data */

use num_traits::Num;
use numpy::{
    IntoPyArray, PyArray1,
    ndarray::{Array, Array3},
};
use pyo3::prelude::*;

#[derive(Clone)]
pub struct RasterInfo {
    pub ncols: usize,
    pub nrows: usize,
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
    pub epsg: Option<u16>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct RawRasterInfo {
    ncols: usize,
    nrows: usize,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    xres: f64,
    yres: f64,
    has_extent: bool,
    tap: bool,
    epsg: Option<u16>,
}

impl<'py> FromPyObject<'py> for RasterInfo {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let raw: RawRasterInfo = ob.extract()?;
        let info = RasterInfo::from(raw);
        Ok(info)
    }
}

impl RasterInfo {
    #[inline]
    fn from(raw: RawRasterInfo) -> Self {
        let mut info = RasterInfo {
            ncols: raw.ncols,
            nrows: raw.nrows,
            xmin: raw.xmin,
            xmax: raw.xmax,
            ymin: raw.ymin,
            ymax: raw.ymax,
            xres: raw.xres,
            yres: raw.yres,
            epsg: raw.epsg,
        };

        let has_res = info.xres != 0.0;
        let has_shape = info.nrows != 0;

        // extent by half pixel if custom extent not provided
        if !raw.has_extent && !raw.tap && has_res {
            info.xmin -= info.xres / 2.0;
            info.xmax += info.xres / 2.0;
            info.ymin -= info.yres / 2.0;
            info.ymax += info.yres / 2.0;
        }

        if !has_res {
            info.resolution();
        } else if raw.tap && has_res {
            info.xmin = (info.xmin / info.xres).floor() * info.xres;
            info.xmax = (info.xmax / info.xres).ceil() * info.xres;
            info.ymin = (info.ymin / info.yres).floor() * info.yres;
            info.ymax = (info.ymax / info.yres).ceil() * info.yres;
        }

        if !has_shape {
            info.shape();
        }

        info
    }

    #[inline]
    fn shape(&mut self) {
        self.nrows = (0.5 + (self.ymax - self.ymin) / self.yres) as usize;
        self.ncols = (0.5 + (self.xmax - self.xmin) / self.xres) as usize
    }

    #[inline]
    fn resolution(&mut self) {
        self.xres = (self.xmax - self.xmin) / self.ncols as f64;
        self.yres = (self.ymax - self.ymin) / self.nrows as f64;
    }

    pub fn build_raster<T>(&self, bands: usize, background: T) -> Array3<T>
    where
        T: Num + Copy,
    {
        Array3::from_elem((bands, self.nrows, self.ncols), background)
    }

    // construct coordinates for xarray (start from pixel's center)
    pub fn make_coordinates<'py>(&self, py: Python<'py>) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>) {
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
