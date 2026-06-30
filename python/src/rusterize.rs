use crate::{
    encoding::pyarray::{PyOutput, Pythonize},
    geo::{parse_geometry::ParsedGeometry, raster::RawRasterInfo},
    prelude::*,
};
use num_traits::One;
use numpy::{Element, PyReadonlyArray1};
use polars::prelude::*;
use pyo3::{
    conversion::FromPyObject,
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyAny,
};
use pyo3_polars::PyDataFrame;
use rusterize::prelude::*;

macro_rules! dispatch_rusterize {
    (
        $dtype:expr, $encoding:expr, $py:expr, $ctx:expr,
        [ $( ($str_val:pat, $rust_type:ty) ),* ]
    ) => {
        match ($dtype, $encoding) {
            $(
                ($str_val, "xarray" | "numpy") => rusterize_py_impl::<DenseArray<$rust_type>>($py, $ctx),
                ($str_val, "sparse") => rusterize_py_impl::<SparseArray<$rust_type>>($py, $ctx),
            )*
            _ => unimplemented!("Invalid dtype or encoding provided."),
        }
    };
}

struct Context<'py> {
    geometry: ParsedGeometry,
    raster_info: RasterInfo,
    pixel_fn: PixelFunction,
    pybackground: Option<&'py Bound<'py, PyAny>>,
    df: Option<DataFrame>,
    pyfield: Option<&'py str>,
    pyby: Option<&'py str>,
    pyburn: Option<&'py Bound<'py, PyAny>>,
    opt_flags: OptionalFlags,
}

fn rusterize_py_impl<'py, A>(py: Python<'py>, ctx: Context<'py>) -> PyResult<PyOutput<'py>>
where
    A: ArrayBuilder + Pythonize,
    A::Dtype: Default + Element + for<'a> FromPyObject<'a, 'py>,
{
    let background = ctx
        .pybackground
        .and_then(|inner| inner.extract().ok())
        .unwrap_or_default();

    let prepared = match &ctx.df {
        Some(df) => {
            let mut exprs: Vec<Expr> = Vec::new();
            if let Some(field) = ctx.pyfield {
                exprs.push(col(field).cast(<A::Dtype>::polars_dtype()).alias("field"));
            }
            if let Some(by) = ctx.pyby {
                exprs.push(col(by).cast(DataType::String).alias("by"));
            }
            Some(
                df.clone()
                    .lazy()
                    .select(exprs)
                    .collect()
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
            )
        }
        _ => None,
    };

    let arr: PyReadonlyArray1<A::Dtype>;

    let field = match (&prepared, ctx.pyfield) {
        (Some(df), Some(_)) => FieldSource::Column(df.column("field").unwrap().clone()),
        _ => match ctx.pyburn {
            None => FieldSource::Scalar(<A::Dtype>::one()),
            Some(b) => match b.extract::<A::Dtype>() {
                Ok(scalar) => FieldSource::Scalar(scalar),
                Err(_) => {
                    arr = b.extract::<PyReadonlyArray1<A::Dtype>>()?;
                    FieldSource::Array(arr.as_array())
                }
            },
        },
    };

    // force every geometry to have a corresponding by value, errors if nulls
    let by = match (&prepared, ctx.pyby) {
        (Some(df), Some(_)) => {
            let ca = df.column("by").unwrap().str().unwrap();

            if ca.null_count() > 0 {
                return Err(PyRuntimeError::new_err(
                    "Found nulls in `by` column. Consider droppping them.",
                ));
            }

            let by_vec = ca
                .downcast_iter()
                .flat_map(|a| a.values_iter())
                .map(str::to_owned)
                .collect::<Vec<String>>();
            Some(by_vec)
        }
        _ => None,
    };

    let rctx = RasterizeContext {
        raster_info: ctx.raster_info,
        field,
        by: by.as_deref(),
        pixel_fn: ctx.pixel_fn,
        background,
        all_touched: ctx.opt_flags.all_touched,
    };

    ctx.geometry
        .rasterize::<A>(rctx)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
        .pythonize(py, ctx.opt_flags)
}

#[pyfunction]
#[pyo3(name = "_rusterize")]
#[pyo3(signature = (geometry, raw_raster_info, pypixel_fn, pydf=None, pyfield=None, pyby=None, pyburn=None, pybackground=None, pytouched=false, pyencoding="xarray", pydtype="float64"))]
#[allow(clippy::too_many_arguments)]
fn rusterize_py<'py>(
    py: Python<'py>,
    geometry: ParsedGeometry,
    raw_raster_info: RawRasterInfo,
    pypixel_fn: &'py str,
    pydf: Option<PyDataFrame>,
    pyfield: Option<&'py str>,
    pyby: Option<&'py str>,
    pyburn: Option<&'py Bound<PyAny>>,
    pybackground: Option<&'py Bound<PyAny>>,
    pytouched: bool,
    pyencoding: &str,
    pydtype: &str,
) -> PyResult<PyOutput<'py>> {
    let df: Option<DataFrame> = pydf.map(|inner| inner.into());
    let raster_info = raw_raster_info
        .build(geometry.as_ref())
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    let pixel_fn = pypixel_fn
        .parse::<PixelFunction>()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let opt_flags = OptionalFlags::new(pytouched, pyencoding);

    let ctx = Context {
        geometry,
        raster_info,
        pixel_fn,
        pybackground,
        df,
        pyfield,
        pyby,
        pyburn,
        opt_flags,
    };

    dispatch_rusterize!(
        pydtype,
        pyencoding,
        py,
        ctx,
        [
            ("uint8", u8),
            ("uint16", u16),
            ("uint32", u32),
            ("uint64", u64),
            ("int8", i8),
            ("int16", i16),
            ("int32", i32),
            ("int64", i64),
            ("float32", f32),
            ("float64", f64)
        ]
    )
}

#[pymodule]
#[pyo3(name = "_rusterize")]
fn rusterize_wrap(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusterize_py, m)?)?;
    Ok(())
}
