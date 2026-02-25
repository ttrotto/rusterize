/* Implementation of rusterize and rasterization logics */

use crate::{
    encoding::{
        arrays::{DenseArray, SparseArray},
        writers::{DenseArrayWriter, PixelWriter, SparseArrayWriter, ToSparseArray},
    },
    geo::{edges::LineEdge, parse_geometry::ParsedGeometry, raster::RasterInfo},
    prelude::{Dense, OptFlags, PolarsHandler, Sparse},
    rasterization::{
        burn_geometry::Burn,
        burners::{AllTouched, AllTouchedCached, LineBurnStrategy, Standard},
        pixel_functions::PixelFn,
    },
};
use fixedbitset::FixedBitSet;
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
    xmin: isize,
    ymin: isize,
}

impl PixelCache {
    pub fn new(linedges: &[LineEdge]) -> Self {
        let (xmin, ymin, xmax, ymax) = linedges.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(xmin, ymin, xmax, ymax), edge| {
                (
                    xmin.min(edge.x0).min(edge.x1),
                    ymin.min(edge.y0).min(edge.y1),
                    xmax.max(edge.x0).max(edge.x1),
                    ymax.max(edge.y0).max(edge.y1),
                )
            },
        );

        let width = (xmax.floor() - xmin.floor()) as usize + 1;
        let length = (ymax.floor() - ymin.floor()) as usize + 1;

        Self {
            bits: FixedBitSet::with_capacity(width * length),
            width,
            xmin: xmin as isize,
            ymin: ymin as isize,
        }
    }

    #[inline]
    fn unravel_index(&self, x: usize, y: usize) -> usize {
        let local_x = (x as isize - self.xmin) as usize;
        let local_y = (y as isize - self.ymin) as usize;
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
    pub geometry: ParsedGeometry,
    pub field: Column,
    pub pixel_fn: PixelFn<N>,
    pub background: N,
    pub opt_flags: OptFlags,
}

macro_rules! dispatch_burn {
    ($all_touched:expr, $dedup:expr, $func:ident, $ctx:expr, $writer:expr $(, $ext:expr)*) => {
        match ($all_touched, $dedup) {
            (true, true)   => $func::<N, _, AllTouchedCached>($ctx, $writer $(, $ext)*),
            (true, false)  => $func::<N, _, AllTouched>($ctx, $writer $(, $ext)*),
            (false, _)  => $func::<N, _, Standard>($ctx, $writer $(, $ext)*),
        }
    };
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
        let all_touched = ctx.opt_flags.with_all_touched();
        let dedup = ctx.opt_flags.requires_deduplication();

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

                        dispatch_burn!(all_touched, dedup, process_multi, &ctx, &mut writer, &idxs);

                        by.get(group_idx as usize).unwrap().to_string()
                    })
                    .collect_into_vec(&mut band_names);

                DenseArray::new(raster, band_names, ctx.raster_info)
            }
            None => {
                let band_names = vec![String::from("band_1")];
                let mut raster = ctx.raster_info.build_raster(1, ctx.background);
                let mut writer = DenseArrayWriter::new(raster.index_axis_mut(Axis(0), 0), ctx.pixel_fn);

                dispatch_burn!(all_touched, dedup, process_single, &ctx, &mut writer);

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
        let all_touched = ctx.opt_flags.with_all_touched();
        let dedup = ctx.opt_flags.requires_deduplication();

        match by {
            Some(by) => {
                let (n_groups, group_idx) = get_groups(by);
                let mut writers: Vec<SparseArrayWriter<N>> = Vec::with_capacity(n_groups);

                group_idx
                    .into_par_iter()
                    .map(|(group_idx, idxs)| {
                        let band_name = by.get(group_idx as usize).unwrap().to_string();
                        let mut writer = SparseArrayWriter::new(band_name);

                        dispatch_burn!(all_touched, dedup, process_multi, &ctx, &mut writer, &idxs);

                        writer
                    })
                    .collect_into_vec(&mut writers);

                writers.finish(ctx)
            }
            None => {
                let mut writer = SparseArrayWriter::new(String::from("band_1"));

                dispatch_burn!(all_touched, dedup, process_single, &ctx, &mut writer);

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
                geom.burn::<S>(&ctx.raster_info, fv, writer, ctx.background)
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
            geom.burn::<S>(&ctx.raster_info, fv, writer, ctx.background)
        }
    }
}
