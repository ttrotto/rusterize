/* Implementation of rusterize and rasterization logics */

use crate::{
    encoding::{
        arrays::{DenseArray, SparseArray},
        pyarrays::Pythonize,
        writers::{DenseArrayWriter, PixelWriter, SparseArrayWriter, ToSparseArray},
    },
    geo::{raster::RasterInfo, validate::validate_geometries},
    prelude::{Dense, PolarsHandler, Sparse},
    rasterization::{
        pixel_functions::PixelFn, prepare_dataframe::cast_df,
        rasterize_geometry::rasterize_geometry,
    },
};
use geo_types::Geometry;
use ndarray::Axis;
use num_traits::Num;
use numpy::Element;
use polars::prelude::*;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

pub struct RasterizeConfig<N> {
    pub raster_info: RasterInfo,
    pub geom: Vec<Geometry>,
    pub field: Column,
    pub pixel_fn: PixelFn<N>,
    pub background: N,
}

#[allow(clippy::too_many_arguments)]
pub fn rusterize_impl<T, R>(
    geometry: Vec<Geometry>,
    mut raster_info: RasterInfo,
    pixel_fn: PixelFn<T>,
    background: T,
    df: Option<DataFrame>,
    field_name: Option<&str>,
    by_name: Option<&str>,
    burn_value: T,
) -> R::Output
where
    T: Num + PolarsHandler,
    R: Rasterize<T>,
    R::Output: Pythonize,
{
    // validate geometries
    let (good_geom, good_df) = validate_geometries(geometry, df, &mut raster_info);

    // extract column from dataframe (cloning is cheap)
    let casted = cast_df(good_df, field_name, by_name, burn_value, good_geom.len());
    let field = casted.column("field_casted").unwrap().clone();
    let by = casted.column("by_str").ok().and_then(|by| by.str().ok());

    // main
    let config = RasterizeConfig {
        raster_info,
        geom: good_geom,
        field,
        pixel_fn,
        background,
    };

    R::rasterize(config, by)
}

// rasterization logics
pub trait Rasterize<N> {
    type Output;

    fn rasterize(config: RasterizeConfig<N>, by: Option<&ChunkedArray<StringType>>)
    -> Self::Output;
}

impl<N> Rasterize<N> for Dense
where
    N: Num + PolarsHandler + Copy + Element,
{
    type Output = DenseArray<N>;

    fn rasterize(
        config: RasterizeConfig<N>,
        by: Option<&ChunkedArray<StringType>>,
    ) -> Self::Output {
        match by {
            Some(by) => {
                let (n_groups, group_idx) = get_groups(by);
                let mut band_names: Vec<String> = Vec::with_capacity(n_groups);
                let mut raster = config.raster_info.build_raster(n_groups, config.background);

                raster
                    .outer_iter_mut()
                    .into_par_iter()
                    .zip(group_idx.into_par_iter())
                    .map(|(band, (group_idx, idxs))| {
                        let mut writer = DenseArrayWriter::new(band, config.pixel_fn);

                        process_multi(&config, &mut writer, &idxs);

                        by.get(group_idx as usize).unwrap().to_string()
                    })
                    .collect_into_vec(&mut band_names);

                DenseArray::new(raster, band_names, config.raster_info)
            }
            None => {
                let band_names = vec![String::from("band_1")];
                let mut raster = config.raster_info.build_raster(1, config.background);
                let mut writer =
                    DenseArrayWriter::new(raster.index_axis_mut(Axis(0), 0), config.pixel_fn);

                process_single(&config, &mut writer);

                DenseArray::new(raster, band_names, config.raster_info)
            }
        }
    }
}

impl<N> Rasterize<N> for Sparse
where
    N: Num + PolarsHandler + Copy + Element,
{
    type Output = SparseArray<N>;

    fn rasterize(
        config: RasterizeConfig<N>,
        by: Option<&ChunkedArray<StringType>>,
    ) -> Self::Output {
        match by {
            Some(by) => {
                let (n_groups, group_idx) = get_groups(by);
                let mut writers: Vec<SparseArrayWriter<N>> = Vec::with_capacity(n_groups);

                group_idx
                    .into_par_iter()
                    .map(|(group_idx, idxs)| {
                        let band_name = by.get(group_idx as usize).unwrap().to_string();
                        let mut writer = SparseArrayWriter::new(band_name);

                        process_multi(&config, &mut writer, &idxs);

                        writer
                    })
                    .collect_into_vec(&mut writers);

                writers.finish(config)
            }
            None => {
                let mut writer = SparseArrayWriter::new(String::from("band_1"));

                process_single(&config, &mut writer);

                writer.finish(config)
            }
        }
    }
}

// wrapper functions for rasterization
fn get_groups(by: &ChunkedArray<StringType>) -> (usize, GroupsIdx) {
    let groups = by.group_tuples(true, true).expect("No groups found!");
    (groups.len(), groups.into_idx())
}

fn process_single<N, W>(config: &RasterizeConfig<N>, writer: &mut W)
where
    N: Num + PolarsHandler + Copy,
    W: PixelWriter<N>,
{
    config
        .field
        .phys_iter()
        .zip(&config.geom)
        .for_each(|(field_value, geom)| {
            if let Some(fv) = N::from_anyvalue(field_value) {
                // process only non-empty field values
                rasterize_geometry(&config.raster_info, geom, fv, writer, config.background)
            }
        });
}

fn process_multi<N, W>(config: &RasterizeConfig<N>, writer: &mut W, idxs: &[u32])
where
    N: Num + PolarsHandler + Copy,
    W: PixelWriter<N>,
{
    for &i in idxs.iter() {
        if let (Some(fv), Some(geom)) = {
            let anyvalue = config.field.get(i as usize).unwrap();
            (N::from_anyvalue(anyvalue), config.geom.get(i as usize))
        } {
            // process only non-empty field values
            rasterize_geometry(&config.raster_info, geom, fv, writer, config.background);
        }
    }
}
