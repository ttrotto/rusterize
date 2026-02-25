/* Structure to contain information on raster data */

use geo::BoundingRect;
use geo_types::{Geometry, Rect, coord};
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
pub struct RawRasterInfo {
    ncols: usize,
    nrows: usize,
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
    xres: f64,
    yres: f64,
    with_user_extent: bool,
    tap: bool,
    epsg: Option<u16>,
}

impl RasterInfo {
    pub fn from(raw: RawRasterInfo, geoms: &[Geometry<f64>]) -> Self {
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

        if info.xmin.is_infinite() {
            // list or numpy.ndarray do not carry bounding information
            let bounds = geoms.iter().fold(None, |acc, geom| {
                let bounds = geom.bounding_rect();

                match (acc, bounds) {
                    (None, None) => None,
                    (None, Some(r)) | (Some(r), None) => Some(r),
                    (Some(r1), Some(r2)) => Some(Rect::new(
                        coord! { x: r1.min().x.min(r2.min().x), y: r1.min().y.min(r2.min().y) },
                        coord! { x: r1.max().x.max(r2.max().x), y: r1.max().y.max(r2.max().y) },
                    )),
                }
            });

            if let Some(b) = bounds {
                info.xmin = b.min().x;
                info.ymin = b.min().y;
                info.xmax = b.max().x;
                info.ymax = b.max().y;
            } else {
                panic!("Cannot infer bounding box from geometry.")
            }
        }

        let has_res = info.xres != 0.0;
        let has_shape = info.nrows != 0;

        // extent by half pixel if custom extent not provided
        if !raw.with_user_extent && !raw.tap && has_res {
            info.xmin -= info.xres / 2.0;
            info.xmax += info.xres / 2.0;
            info.ymin -= info.yres / 2.0;
            info.ymax += info.yres / 2.0;
        }

        if !has_res {
            info.assign_resolution();
        } else if raw.tap && has_res {
            info.xmin = (info.xmin / info.xres).floor() * info.xres;
            info.xmax = (info.xmax / info.xres).ceil() * info.xres;
            info.ymin = (info.ymin / info.yres).floor() * info.yres;
            info.ymax = (info.ymax / info.yres).ceil() * info.yres;
        }

        if !has_shape {
            info.assign_shape();
        }

        info
    }

    #[inline]
    fn assign_shape(&mut self) {
        self.nrows = (0.5 + (self.ymax - self.ymin) / self.yres) as usize;
        self.ncols = (0.5 + (self.xmax - self.xmin) / self.xres) as usize
    }

    #[inline]
    fn assign_resolution(&mut self) {
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
