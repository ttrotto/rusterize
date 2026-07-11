use std::cmp::Ordering;

use crate::error::{RusterizeError, RusterizeResult};
use geo::{BoundingRect, Geometry, Rect, coord};
use ndarray::Array3;
use num_traits::Num;

/// Contains the spatial information associated with the burned [`geo::Geometry`].
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

impl RasterInfo {
    pub(crate) fn build_raster<N>(&self, bands: usize, background: N) -> Array3<N>
    where
        N: Num + Copy,
    {
        Array3::from_elem((bands, self.nrows, self.ncols), background)
    }
}

/// Builder for a [`RasterInfo`] instance.
/// If extent is not provided, it can be inferred from the [`geo::Geometry`] when building it.
/// In this case, a half-pixel buffer is applied to avoid missing points on the border.
/// The logics dictating the final spatial properties of the rasterized geometries follow those of GDAL.
#[derive(Default)]
pub struct RasterInfoBuilder {
    shape: Option<[usize; 2]>,
    extent: Option<[f64; 4]>,
    resolution: Option<[f64; 2]>,
    tap: bool,
    epsg: Option<u16>,
}

impl RasterInfoBuilder {
    pub fn new() -> Self {
        RasterInfoBuilder::default()
    }

    /// Build into a [`RasterInfo`] with user-defined extent.
    pub fn build(self) -> RusterizeResult<RasterInfo> {
        match self.extent {
            Some(extent) => {
                let is_unspecified_extent = extent.iter().all(|x| matches!(x.total_cmp(&0.0), Ordering::Equal));
                if is_unspecified_extent {
                    return Err(RusterizeError::ValueError("Unspecified extent (all zeros)."));
                }
                self.finalize(extent, false)
            }
            None => Err(RusterizeError::RuntimeError(
                "Extent must be provided for construction. \
                Use `build_with()` to infer extent from geometries.",
            )),
        }
    }

    /// Same as `build`, but infer extent from the geometry.
    pub fn build_with(self, geoms: &[Geometry<f64>]) -> RusterizeResult<RasterInfo> {
        if self.extent.is_some() {
            return Err(RusterizeError::RuntimeError(
                "Extent must be inferred from geometries for construction. \
                Use `build()` to provide a custom extent.",
            ));
        }

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
            self.finalize([b.min().x, b.min().y, b.max().x, b.max().y], true)
        } else {
            Err(RusterizeError::RuntimeError("Cannot infer bounding box from geometry."))
        }
    }

    fn finalize(
        self,
        [mut xmin, mut ymin, mut xmax, mut ymax]: [f64; 4],
        inferred: bool,
    ) -> RusterizeResult<RasterInfo> {
        if self.shape.is_none() && self.resolution.is_none() {
            return Err(RusterizeError::ValueError(
                "Must set at least one of `shape` or `resolution`",
            ));
        }
        if self.shape.is_some() && self.resolution.is_some() {
            return Err(RusterizeError::ValueError(
                "Shape and resolution are mutually exclusive; provide only one",
            ));
        }
        let has_shape = self.shape.is_some();
        let has_res = self.resolution.is_some();
        let [mut nrows, mut ncols] = self.shape.unwrap_or_default();
        let [mut xres, mut yres] = self.resolution.unwrap_or_default();

        if has_shape && (nrows == 0 || ncols == 0) {
            return Err(RusterizeError::ValueError("Shape values must be > 0."));
        }

        if has_res && (xres <= 0.0 || yres <= 0.0) {
            return Err(RusterizeError::ValueError("Resolution values must be > 0."));
        }

        if inferred && !self.tap && has_res {
            xmin -= xres / 2.0;
            xmax += xres / 2.0;
            ymin -= yres / 2.0;
            ymax += yres / 2.0;
        }

        if !has_res {
            xres = (xmax - xmin) / ncols as f64;
            yres = (ymax - ymin) / nrows as f64;
        } else if self.tap {
            xmin = (xmin / xres).floor() * xres;
            xmax = (xmax / xres).ceil() * xres;
            ymin = (ymin / yres).floor() * yres;
            ymax = (ymax / yres).ceil() * yres;
        }

        if !has_shape {
            nrows = (0.5 + (ymax - ymin) / yres) as usize;
            ncols = (0.5 + (xmax - xmin) / xres) as usize;
        }

        Ok(RasterInfo {
            ncols,
            nrows,
            xmin,
            xmax,
            ymin,
            ymax,
            xres,
            yres,
            epsg: self.epsg,
        })
    }

    pub fn shape(mut self, nrows: usize, ncols: usize) -> Self {
        self.shape = Some([nrows, ncols]);
        self
    }

    pub fn extent(mut self, xmin: f64, ymin: f64, xmax: f64, ymax: f64) -> Self {
        self.extent = Some([xmin, ymin, xmax, ymax]);
        self
    }

    pub fn resolution(mut self, xres: f64, yres: f64) -> Self {
        self.resolution = Some([xres, yres]);
        self
    }

    pub fn with_target_aligned_pixel(mut self) -> Self {
        self.tap = true;
        self
    }

    pub fn epsg(mut self, epsg: u16) -> Self {
        self.epsg = Some(epsg);
        self
    }
}
