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
    geometry: Vec<Geometry>,
    ras_info: Raster,
    pixel_fn: PixelFn,
    background: f64,
    field: String,
    by: String,
}

impl Rusterize {
    pub fn new(
        geometry: Vec<Geometry>,
        ras_info: Raster,
        pixel_fn: PixelFn,
        background: f64,
        field: String,
        by: String,
    ) -> Self {
        Self {
            geometry,
            ras_info,
            pixel_fn,
            background,
            field,
            by,
        }
    }

    // polars expression
    fn map_col_rusterize(
        &self,
        raster: &mut Array3<f64>,
        band_idx: &mut usize,
    ) -> Expr {
        as_struct(vec![col(&self.field), col("idx")]).map(
            |s| {
                let sc = s.struct_()?;
                let idx_sc = sc.field_by_name("idx")?.u32()?;
                let field_sc = sc.field_by_name(self.field.as_str())?.f64()?;
                // apply function to each field-idx pair
                let result: BooleanChunked = field_sc
                    .into_no_null_iter()
                    .zip(idx_sc.into_no_null_iter())
                    .map(|(field_value, idx)| {
                        // let uidx = idx as usize;
                        rasterize_polygon(
                            &self.ras_info,
                            &self.geometry[idx as usize],
                            &field_value,
                            &mut raster.index_axis_mut(Axis(0), *band_idx),
                            &self.pixel_fn,
                        )
                    })
                    .collect();
                 // band index for group by
                if !self.by.is_empty() {
                    *band_idx += 1
                }
                Ok(Some(Column::from(result.into_series())))
            },
            // define output type
            GetOutput::from_type(DataType::Boolean),
        )
    }

    fn rusterize_rust(
        self,
        df: DataFrame,
    ) -> Array3<f64> {
        // polars does not allow Geometry, so add index for query
        let df_lazy = df.lazy().with_row_index(PlSmallStr::from("idx"), None);

        // index for building multiband raster
        let mut band_idx: usize = 0;

        if self.by.is_empty() {
            // build raster
            let mut raster = self.ras_info.build_raster(1);

            // rasterize each polygon iteratively
            df_lazy
                .with_columns([self.map_col_rusterize(&mut raster, &mut band_idx)])
                .collect()
                .map_err(PolarsError::from)
                .unwrap();

            // return
            raster
        } else {
            // determine groups while keeping order
            let groups = df_lazy.group_by_stable([col(&self.by)]);
            let bands = groups.clone().head(Some(1)).collect().unwrap().height();

            // build raster
            let mut raster = self.ras_info.build_raster(bands);

            // rasterize each polygon iteratively by group
            groups
                .agg([self.map_col_rusterize(&mut raster, &mut band_idx)])
                .collect()
                .map_err(PolarsError::from)
                .unwrap();

            // return
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
    let rclass = Rusterize::new(geometry, raster_info, pixel_fn, background, field, by);
    let ret = rclass.rusterize_rust(df);
    Ok(ret.to_pyarray(py))
}

fn rusterize(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
