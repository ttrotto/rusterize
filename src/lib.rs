#![feature(extract_if)]
extern crate blas_src;

mod structs {
    pub mod edge;
    pub mod raster;
}
mod edgelist;
mod pixel_functions;
mod rasterize_polygon;

use crate::pixel_functions::{set_pixel_function, PixelFn};
use crate::rasterize_polygon::rasterize_polygon;
use geo_types::Geometry;
use numpy::{
    ndarray::{Array3, Axis},
    PyArray3, ToPyArray,
};
use polars::prelude::*;
use py_geo_interface::wrappers::f64::AsGeometryVec;
use pyo3::{
    prelude::*,
    types::{PyAny, PyString},
};
use pyo3_polars::PyDataFrame;
use structs::raster::Raster;

struct Rusterize {
    df: DataFrame,
    geometry: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: String,
    by: String,
}

impl Rusterize {
    fn new(
        df: DataFrame,
        geometry: Vec<Geometry>,
        ras_info: Raster,
        pixel_fn: PixelFn,
        background: f64,
        field: String,
        by: String,
    ) -> Self {
        Self {
            df,
            geometry,
            ras_info,
            pixel_fn,
            background,
            field,
            by,
        }
    }

   fn rusterize_sequential(self) -> Array3<f64> {
        // extract field values as iterator - except null
        let field_iter = self.df
            .column(&self.field.as_str())
            .as_ref()
            .unwrap()
            .f64()
            .unwrap();

        if self.by.is_empty() {
            // init raster
            let mut raster = self.ras_info.build_raster(1);

            // iter rasterize
            field_iter
                .into_no_null_iter()
                .zip(self.geometry.iter())
                .for_each(|(field_value, geom)| {
                    rasterize_polygon(
                        &self.ras_info,
                        geom,
                        &field_value,
                        &mut raster.index_axis_mut(Axis(0), 0),
                        &self.pixel_fn,
                    )
                });
            raster
        } else {
            // get number of groups for multiband raster
            let bands = self.df
                .lazy()
                .group_by(self.by)
                .head(Some(1))  // extract 1st row of each for proxy
                .collect()
                .unwrap()
                .height();

            // init raster
            let mut raster = self.ras_info.build_raster(bands);

            // iter rasterize
            field_iter
                .into_no_null_iter()
                .zip(self.geometry.iter())
                .enumerate()
                .for_each(|(idx, (field_value, geom))| {
                    rasterize_polygon(
                        &self.ras_info,
                        &geom,
                        &field_value,
                        &mut raster.index_axis_mut(Axis(0), idx),
                        &self.pixel_fn,
                    )
                });
            raster
        }
    }
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pydf: PyDataFrame,
    pygeometry: &PyAny,
    pyinfo: &PyAny,
    pypixel_fn: &PyString,
    pybackground: &PyAny,
    pyfield: Option<&PyString>,
    pyby: Option<&PyString>,
) -> PyResult<&'py PyArray3<f64>> {
    // extract dataframe
    let df: DataFrame = pydf.into();

    // extract geometries
    let geometry = pygeometry.as_geometry_vec()?.0;

    // extract raster information
    let raster_info = Raster::from(&pyinfo);

    // extract function arguments
    let fun = pypixel_fn.to_str()?;
    let pixel_fn = set_pixel_function(fun);
    let background = pybackground.extract::<f64>()?;
    let field = match pyfield {
        Some(inner) => inner.to_string(),
        None => String::new(),
    };
    let by = match pyby {
        Some(inner) => inner.to_string(),
        None => String::new(),
    };

    // rusterize
    let ret = py.allow_threads(|| {
        let rclass = Rusterize::new(df, geometry, raster_info, pixel_fn, background, field, by);
        rclass.rusterize_sequential();
    });
    Ok(ret.to_pyarray(py))
}

fn rusterize(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}