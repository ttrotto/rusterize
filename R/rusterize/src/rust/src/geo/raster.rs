use crate::conversion::FromR;
use geo::Geometry;
use rusterize::prelude::{RasterInfo, RasterInfoBuilder};
use savvy::{Error as SavvyError, ListSexp, Result as SavvyResult, Sexp, TypedSexp, savvy_err};

/// Build a [`rusterize::RasterInfo`] from input spatial information.
pub(crate) fn build_raster_info(raw: ListSexp, geoms: &[Geometry<f64>]) -> SavvyResult<RasterInfo> {
    let mut builder = RasterInfoBuilder::new();
    let mut with_user_extent = false;

    for (name, sexp) in raw.iter() {
        match (name, sexp.into_typed()) {
            ("shape", TypedSexp::List(ls)) => {
                let shape = list_sexp_to_vec::<usize>(ls)?;
                builder = builder.shape(shape[0], shape[1]);
            }
            ("resolution", TypedSexp::List(ls)) => {
                let resolution = list_sexp_to_vec::<f64>(ls)?;
                builder = builder.resolution(resolution[0], resolution[1]);
            }
            ("tap", TypedSexp::Logical(l)) if l.iter().next() == Some(true) => {
                builder = builder.with_target_align_pixel();
            }
            ("epsg", TypedSexp::Integer(i)) => {
                let maybe_epsg = i.iter().next().copied();

                if let Some(epsg) = maybe_epsg {
                    if epsg > 0 && epsg <= u16::MAX as i32 {
                        builder = builder.epsg(epsg as u16);
                    } else {
                        return Err(savvy_err!("Expected a positive integer, got {}", epsg));
                    }
                }
            }
            ("extent", TypedSexp::List(ls)) if !ls.is_empty() => {
                with_user_extent = true;
                let extent = list_sexp_to_vec::<f64>(ls)?;
                builder = builder.extent(extent[0], extent[1], extent[2], extent[3]);
            }
            _ => (),
        }
    }

    if with_user_extent {
        Ok(builder.build()?)
    } else {
        Ok(builder.build_with(geoms)?)
    }
}

pub(crate) fn list_sexp_to_vec<T>(ls: ListSexp) -> SavvyResult<Vec<T>>
where
    FromR<T>: TryFrom<Sexp, Error = SavvyError>,
{
    Ok(ls
        .values_iter()
        .map(FromR::<T>::try_from)
        .collect::<SavvyResult<Vec<FromR<T>>>>()?
        .into_iter()
        .map(|wrap| wrap.0)
        .collect::<Vec<T>>())
}
