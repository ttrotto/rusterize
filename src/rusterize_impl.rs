/*
Implementation of rusterize
*/
use crate::{
    geom::validate::validate_geometries, pixel_functions::PixelFn, prelude::PolarsHandler,
    rasterize_geometry::rasterize, structs::raster::RasterInfo,
};
use geo_types::Geometry;
use ndarray::{Array3, Axis};
use num_traits::Num;
use polars::prelude::*;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

#[allow(clippy::too_many_arguments)]
pub fn rusterize_impl<T>(
    geometry: Vec<Geometry>,
    raster_info: &mut RasterInfo,
    pixel_fn: PixelFn<T>,
    background: T,
    df: Option<DataFrame>,
    field_name: Option<&str>,
    by_name: Option<&str>,
    burn_value: T,
) -> (Array3<T>, Vec<String>)
where
    T: Num + Copy + PolarsHandler + Literal + Send + Sync,
{
    // validate geometries
    let (good_geom, df) = validate_geometries(geometry, df, raster_info);

    // extract `field` and `by`
    let casted: DataFrame;
    let (field, by) = match df {
        None => (
            // case 1: create a dummy `field`
            &burn_value.into_column(good_geom.len()),
            None,
        ),
        Some(df) => {
            let mut lf = df.lazy();
            match (field_name, by_name) {
                (Some(field_name), Some(by_name)) => {
                    // case 2: both `field` and `by` specified
                    lf = lf.with_columns([
                        col(field_name)
                            .cast(T::polars_dtype())
                            .alias("field_casted"),
                        col(by_name).cast(DataType::String).alias("by_str"),
                    ]);
                }
                (Some(field_name), None) => {
                    // case 3: only `field` specified
                    lf = lf.with_column(
                        col(field_name)
                            .cast(T::polars_dtype())
                            .alias("field_casted"),
                    );
                }
                (None, Some(by_name)) => {
                    // case 4: only `by` specified
                    lf = lf.with_columns([
                        lit(burn_value).alias("field_casted"), // dummy `field`
                        col(by_name).cast(DataType::String).alias("by_str"),
                    ]);
                }
                (None, None) => {
                    // case 5: neither `field` nor `by` specified
                    lf = lf.with_columns([lit(burn_value).alias("field_casted")])
                }
            }

            // collect casted dataframe
            casted = lf.collect().unwrap();

            (
                casted.column("field_casted").unwrap(),
                casted.column("by_str").ok().and_then(|col| col.str().ok()),
            )
        }
    };

    // main
    let mut raster: Array3<T>;
    let mut band_names: Vec<String> = vec![String::from("band1")];
    match by {
        Some(by) => {
            // get groups
            let groups = by.group_tuples(true, false).expect("No groups found!");
            let n_groups = groups.len();
            let group_idx = groups.into_idx();

            // multiband raster
            raster = raster_info.build_raster(n_groups, background);
            raster
                .outer_iter_mut()
                .into_par_iter()
                .zip(group_idx.into_par_iter())
                .map(|(mut band, (group_idx, idxs))| {
                    // rasterize polygons
                    for &i in idxs.iter() {
                        if let (Some(fv), Some(geom)) = {
                            let anyvalue = field.get(i as usize).unwrap();
                            (T::from_anyvalue(anyvalue), good_geom.get(i as usize))
                        } {
                            // process only non-empty field values
                            rasterize(raster_info, geom, &fv, &mut band, &pixel_fn, &background);
                        }
                    }
                    // band name
                    by.get(group_idx as usize).unwrap().to_string()
                })
                .collect_into_vec(&mut band_names)
        }
        None => {
            // singleband raster
            raster = raster_info.build_raster(1, background);

            // rasterize polygons
            field
                .phys_iter()
                .zip(good_geom)
                .for_each(|(field_value, geom)| {
                    if let Some(fv) = T::from_anyvalue(field_value) {
                        // process only non-empty field values
                        rasterize(
                            raster_info,
                            &geom,
                            &fv,
                            &mut raster.index_axis_mut(Axis(0), 0),
                            &pixel_fn,
                            &background,
                        )
                    }
                });
        }
    }

    (raster, band_names)
}
