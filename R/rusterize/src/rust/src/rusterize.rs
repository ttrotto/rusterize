use crate::{
    conversion::FromR,
    encoding::rarrays::{RArray, RArrayTraits},
    geo::{parse_geometry::parse_geometry, raster::build_raster_info},
};
use geo::Geometry;
use num_traits::One;
use rusterize::prelude::*;
use savvy::{
    Error as SavvyError, IntegerSexp, ListSexp, NumericSexp, Sexp, savvy, savvy_err, sexp::na::NotAvailableValue,
};

fn rusterize_r_impl<A>(ctx: Context) -> savvy::Result<RArray>
where
    A: ArrayBuilder + RArrayTraits + 'static,
    FromR<A::Dtype>: TryFrom<Sexp, Error = SavvyError>,
    FromR<Vec<A::Dtype>>: TryFrom<NumericSexp, Error = SavvyError>,
    A::Dtype: NotAvailableValue,
{
    let background = if ctx.rbackground.is_scalar_na() {
        <A::Dtype>::na()
    } else {
        FromR::<A::Dtype>::try_from(ctx.rbackground)?.0
    };

    let source;
    let field = match ctx.rfield.or(ctx.rburn) {
        Some(v) => {
            source = FromR::<Vec<A::Dtype>>::try_from(v)?.0;
            if source.len() == 1 {
                FieldSource::Scalar(source[0])
            } else {
                FieldSource::from(&source)
            }
        }
        None => FieldSource::Scalar(<A::Dtype>::one()),
    };

    // `by` is an unclassed factor: 1-based integer codes + `levels`. NA becomes an out-of-range code, rejected by the core
    let by = match ctx.rby {
        Some(f) => {
            let labels = f
                .get_attrib("levels")?
                .ok_or_else(|| savvy_err!("`by` must be integer codes with a `levels` attribute."))?;
            let labels = FromR::<Vec<String>>::try_from(labels)?.0;
            let codes: Vec<u32> = IntegerSexp::try_from(f)?
                .iter()
                .map(|&c| c.checked_sub(1).map_or(u32::MAX, |c| c as u32))
                .collect();
            Some((codes, labels))
        }
        None => None,
    };

    let rctx = RasterizeContext {
        raster_info: ctx.raster_info,
        field,
        by: by.as_ref().map(|(codes, labels)| ByGroups { codes, labels }),
        pixel_fn: ctx.pixel_fn,
        background,
        all_touched: ctx.all_touched,
    };

    let array = ctx.geometry.rasterize::<A>(rctx).map_err(|e| savvy_err!("{}", e))?;
    Ok(RArray(Box::new(array)))
}

struct Context {
    geometry: Vec<Geometry<f64>>,
    raster_info: RasterInfo,
    pixel_fn: PixelFunction,
    rbackground: Sexp,
    rfield: Option<NumericSexp>,
    rby: Option<Sexp>,
    rburn: Option<NumericSexp>,
    all_touched: bool,
}

#[savvy]
#[allow(clippy::too_many_arguments)]
fn rusterize_r(
    geometry: ListSexp,
    raw_raster_info: ListSexp,
    rpixel_fn: &str,
    rbackground: Sexp,
    all_touched: bool,
    encoding: &str,
    dtype: &str,
    rfield: Option<NumericSexp>,
    rby: Option<Sexp>,
    rburn: Option<NumericSexp>,
) -> savvy::Result<RArray> {
    let parsed_geometry = parse_geometry(geometry)?;
    let raster_info = build_raster_info(raw_raster_info, &parsed_geometry).map_err(|e| savvy_err!("{}", e))?;
    let pixel_fn = rpixel_fn.parse::<PixelFunction>().map_err(|e| savvy_err!("{}", e))?;

    let ctx = Context {
        geometry: parsed_geometry,
        raster_info,
        pixel_fn,
        rbackground,
        rfield,
        rby,
        rburn,
        all_touched,
    };

    match (dtype, encoding) {
        ("integer", "dense") => rusterize_r_impl::<DenseArray<i32>>(ctx),
        ("double", "dense") => rusterize_r_impl::<DenseArray<f64>>(ctx),
        ("integer", "sparse") => rusterize_r_impl::<SparseArray<i32>>(ctx),
        ("double", "sparse") => rusterize_r_impl::<SparseArray<f64>>(ctx),
        _ => Err(savvy_err!("Unsupported dtype/encoding combination.")),
    }
}
