/* Implementation of rusterize and rasterization logics */

use crate::{
    encoding::{
        arrays::{DenseArray, SparseArray},
        pyarrays::Pythonize,
        writers::{DenseArrayWriter, PixelWriter, SparseArrayWriter, ToSparseArray},
    },
    geo::{edges::LineEdge, raster::RasterInfo},
    prelude::{Dense, OptFlags, PolarsHandler, Sparse},
    rasterization::{
        burn_geometry::Burn,
        burners::{AllTouched, LineBurnStrategy, Standard},
        pixel_functions::PixelFn,
        prepare_dataframe::cast_df,
    },
};
use fixedbitset::FixedBitSet;
use geo_types::Geometry;
use ndarray::Axis;
use num_traits::Num;
use numpy::Element;
use polars::prelude::*;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

// cache pixels for all_touched purposes if pixel_function is "sum" or "count"
// pass 1 -> burn interior and exterior lines with all_touched and record visited pixels
// pass 2 -> fill inner values and skip visited from pass 1
pub struct PixelCache {
    bits: FixedBitSet,
    width: usize,
    min_x: isize,
    min_y: isize,
}

impl PixelCache {
    pub fn new(linedges: &[LineEdge]) -> Self {
        let (min_x, max_x, min_y, max_y) = linedges.iter().fold(
            (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
            |(min_x, max_x, min_y, max_y), edge| {
                (
                    min_x.min(edge.x0).min(edge.x1),
                    max_x.max(edge.x0).max(edge.x1),
                    min_y.min(edge.y0).min(edge.y1),
                    max_y.max(edge.y0).max(edge.y1),
                )
            },
        );

        let width = (max_x.floor() - min_x.floor()) as usize + 1;
        let length = (max_y.floor() - min_y.floor()) as usize + 1;

        Self {
            bits: FixedBitSet::with_capacity(width * length),
            width,
            min_x: min_x as isize,
            min_y: min_y as isize,
        }
    }

    #[inline]
    fn unravel_index(&self, x: usize, y: usize) -> usize {
        let local_x = (x as isize - self.min_x) as usize;
        let local_y = (y as isize - self.min_y) as usize;
        local_y * self.width + local_x
    }

    pub fn insert(&mut self, x: usize, y: usize) -> bool {
        let idx = self.unravel_index(x, y);
        if self.bits.contains(idx) {
            return false;
        }
        self.bits.insert(idx);
        true
    }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        let idx = self.unravel_index(x, y);
        self.bits.contains(idx)
    }
}

pub struct RasterizeContext<N> {
    pub raster_info: RasterInfo,
    geometry: Vec<Geometry>,
    field: Column,
    pub pixel_fn: PixelFn<N>,
    pub background: N,
    opt_flags: OptFlags,
}

#[allow(clippy::too_many_arguments)]
pub fn rusterize_impl<T, R>(
    geometry: Vec<Geometry>,
    raster_info: RasterInfo,
    pixel_fn: PixelFn<T>,
    background: T,
    df: Option<DataFrame>,
    field_name: Option<&str>,
    by_name: Option<&str>,
    burn_value: T,
    opt_flags: OptFlags,
) -> R::Output
where
    T: Num + PolarsHandler,
    R: Rasterize<T>,
    R::Output: Pythonize,
{
    // extract column from dataframe (cloning is cheap)
    let casted = cast_df(df, field_name, by_name, burn_value, geometry.len());
    let field = casted.column("field_casted").unwrap().clone();
    let by = casted.column("by_str").ok().and_then(|by| by.str().ok());

    // main
    let ctx = RasterizeContext {
        raster_info,
        geometry,
        field,
        pixel_fn,
        background,
        opt_flags,
    };

    R::rasterize(ctx, by)
}

// rasterization logics
pub trait Rasterize<N> {
    type Output;

    fn rasterize(ctx: RasterizeContext<N>, by: Option<&ChunkedArray<StringType>>) -> Self::Output;
}

impl<N> Rasterize<N> for Dense
where
    N: Num + PolarsHandler + Copy + Element,
{
    type Output = DenseArray<N>;

    fn rasterize(ctx: RasterizeContext<N>, by: Option<&ChunkedArray<StringType>>) -> Self::Output {
        match by {
            Some(by) => {
                let (n_groups, group_idx) = get_groups(by);
                let mut band_names: Vec<String> = Vec::with_capacity(n_groups);
                let mut raster = ctx.raster_info.build_raster(n_groups, ctx.background);

                raster
                    .outer_iter_mut()
                    .into_par_iter()
                    .zip(group_idx.into_par_iter())
                    .map(|(band, (group_idx, idxs))| {
                        let mut writer = DenseArrayWriter::new(band, ctx.pixel_fn);

                        if ctx.opt_flags.with_all_touched() {
                            process_multi::<N, _, AllTouched>(&ctx, &mut writer, &idxs);
                        } else {
                            process_multi::<N, _, Standard>(&ctx, &mut writer, &idxs);
                        }

                        by.get(group_idx as usize).unwrap().to_string()
                    })
                    .collect_into_vec(&mut band_names);

                DenseArray::new(raster, band_names, ctx.raster_info)
            }
            None => {
                let band_names = vec![String::from("band_1")];
                let mut raster = ctx.raster_info.build_raster(1, ctx.background);
                let mut writer = DenseArrayWriter::new(raster.index_axis_mut(Axis(0), 0), ctx.pixel_fn);

                if ctx.opt_flags.with_all_touched() {
                    process_single::<N, _, AllTouched>(&ctx, &mut writer);
                } else {
                    process_single::<N, _, Standard>(&ctx, &mut writer);
                }

                DenseArray::new(raster, band_names, ctx.raster_info)
            }
        }
    }
}

impl<N> Rasterize<N> for Sparse
where
    N: Num + PolarsHandler + Copy + Element,
{
    type Output = SparseArray<N>;

    fn rasterize(ctx: RasterizeContext<N>, by: Option<&ChunkedArray<StringType>>) -> Self::Output {
        match by {
            Some(by) => {
                let (n_groups, group_idx) = get_groups(by);
                let mut writers: Vec<SparseArrayWriter<N>> = Vec::with_capacity(n_groups);

                group_idx
                    .into_par_iter()
                    .map(|(group_idx, idxs)| {
                        let band_name = by.get(group_idx as usize).unwrap().to_string();
                        let mut writer = SparseArrayWriter::new(band_name);

                        if ctx.opt_flags.with_all_touched() {
                            process_multi::<N, _, AllTouched>(&ctx, &mut writer, &idxs);
                        } else {
                            process_multi::<N, _, Standard>(&ctx, &mut writer, &idxs);
                        }

                        writer
                    })
                    .collect_into_vec(&mut writers);

                writers.finish(ctx)
            }
            None => {
                let mut writer = SparseArrayWriter::new(String::from("band_1"));

                if ctx.opt_flags.with_all_touched() {
                    process_single::<N, _, AllTouched>(&ctx, &mut writer);
                } else {
                    process_single::<N, _, Standard>(&ctx, &mut writer);
                }
                writer.finish(ctx)
            }
        }
    }
}

// wrapper functions for rasterization
fn get_groups(by: &ChunkedArray<StringType>) -> (usize, GroupsIdx) {
    let groups = by.group_tuples(true, true).expect("No groups found!");
    (groups.len(), groups.into_idx())
}

fn process_single<N, W, S>(ctx: &RasterizeContext<N>, writer: &mut W)
where
    N: Num + PolarsHandler + Copy,
    W: PixelWriter<N>,
    S: LineBurnStrategy,
{
    ctx.field
        .phys_iter()
        .zip(&ctx.geometry)
        .for_each(|(field_value, geom)| {
            if let Some(fv) = N::from_anyvalue(field_value) {
                geom.burn::<S>(&ctx.raster_info, fv, writer, ctx.background, &ctx.opt_flags)
            }
        });
}

fn process_multi<N, W, S>(ctx: &RasterizeContext<N>, writer: &mut W, idxs: &[u32])
where
    N: Num + PolarsHandler + Copy,
    W: PixelWriter<N>,
    S: LineBurnStrategy,
{
    for &i in idxs.iter() {
        if let (Some(fv), Some(geom)) = {
            let anyvalue = ctx.field.get(i as usize).unwrap();
            (N::from_anyvalue(anyvalue), ctx.geometry.get(i as usize))
        } {
            geom.burn::<S>(&ctx.raster_info, fv, writer, ctx.background, &ctx.opt_flags)
        }
    }
}
