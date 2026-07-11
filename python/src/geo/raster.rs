use geo::Geometry;
use numpy::{IntoPyArray, PyArray1, ndarray::Array};
use pyo3::prelude::*;
use rusterize::prelude::{RasterInfo, RasterInfoBuilder, RusterizeResult};

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
pub struct RawRasterInfo {
    shape: Option<[usize; 2]>,
    extent: Option<[f64; 4]>,
    resolution: Option<[f64; 2]>,
    tap: bool,
    epsg: Option<u16>,
}

impl RawRasterInfo {
    pub(crate) fn build(self, geoms: &[Geometry<f64>]) -> RusterizeResult<RasterInfo> {
        let mut builder = RasterInfoBuilder::new();

        if let Some(shape) = self.shape {
            builder = builder.shape(shape[0], shape[1]);
        }

        if let Some(resolution) = self.resolution {
            builder = builder.resolution(resolution[0], resolution[1]);
        }

        if let Some(epsg) = self.epsg {
            builder = builder.epsg(epsg);
        }

        if self.tap {
            builder = builder.with_target_align_pixel();
        }

        if let Some(extent) = self.extent {
            builder.extent(extent[0], extent[1], extent[2], extent[3]).build()
        } else {
            builder.build_with(geoms)
        }
    }
}

/// Construct coordinates for xarray (start from pixel's center)
pub(crate) fn make_coordinates<'py>(
    py: Python<'py>,
    info: &RasterInfo,
) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>) {
    let y_coords = Array::range(
        info.ymax - info.yres / 2.0,
        info.ymax - info.nrows as f64 * info.yres,
        -info.yres,
    )
    .into_pyarray(py);
    let x_coords = Array::range(
        info.xmin + info.xres / 2.0,
        info.xmin + info.ncols as f64 * info.xres,
        info.xres,
    )
    .into_pyarray(py);
    (y_coords, x_coords)
}
