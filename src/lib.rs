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

fn rusterize_rust(
    df: DataFrame,
    geometry: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: String,
    by: String,
) -> Array3<f64> {
    // add geometry to a lazyframe
    // polars does not allow Geometry, so it is wrapped around ChunkedArray
    let geom_series = Series::new("geometry".into(), ChunkedArray::from_vec("geometry".into(), geometry));
    let df_lazy = df.lazy().with_column(geom_series.into());

    if by.is_empty() {
        // build raster
        let mut raster = ras_info.build_raster(1);

        // rasterize each polygon iteratively
        df_lazy
            .with_columns(cols([field, "geometry".to_string()]).map(
                |series| {
                    let s = series.struct_()?;
                    let field_value = s.field_by_name(field.as_str())?.f64()?;
                    let geometry = s.field_by_name("geometry")?;
                    let result: ChunkedArray<bool> = field_value
                        .into_iter()
                        .zip(geometry.into_iter())
                        .map(|(value, geom)| match (value, geom) {
                            (Some(value), Some(geom)) => Some(rasterize_polygon(&ras_info, geom, &value, &raster, &pixel_fn))
                        })
                        .collect();
                    Ok(result.into_series())
                },
                GetOutput::from_type(DataType::Boolean),
            ))
            .collect()
            .expect("Rasterization unsuccessful");

        raster
    } else {
        // build raster
    }
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
fn rusterize_py<'py>(
    py: Python<'py>,
    pydf: PyDataFrame,
    pygeometry: PyAny,
    pyinfo: PyAny,
    pypixel_fn: PyString,
    pybackground: PyAny,
    pyfield: Option<PyString>,
    pyby: Option<PyString>,
) -> PyResult<&'py PyArray3<f64>> {
    // extract dataframe
    let df = pydf.into()?;

    // extract geometries
    let geometry = pygeometry.as_geometry_vec()?.0;

    // extract raster information
    let raster_info = Raster::from(&pyinfo);

    // extract function arguments
    let fun = pypixel_fn.to_str()?;
    let pixel_fn = set_pixel_function(fun)?;
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
    let output = py.allow_threads(|| {
        rusterize_rust(df, geometry, raster_info, pixel_fn, background, field, by)
    });
    let ret = output.to_pyarray(py);
    Ok(ret)
}

fn rusterize(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
