use std::collections::HashMap;

use crate::{
    encoding::{
        arrays::{DenseArray, SparseArray},
        writers::{DenseArrayWriter, PixelWriter, SparseArrayWriter, ToSparseArray},
    },
    error::{RusterizeError, RusterizeResult},
    prelude::{RasterDtype, RasterizeContext},
    rasterization::{
        burn_geometry::Burn,
        burners::{AllTouched, AllTouchedCached, LineBurnStrategy, Standard},
    },
};
use geo::Geometry;
use ndarray::{ArrayView1, Axis};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

#[cfg(feature = "polars")]
use polars::prelude::*;

/// Source of values to burn onto a [`DenseArray`] or [`SparseArray`].
#[derive(Clone)]
pub enum FieldSource<'a, N> {
    /// A single constant value to burn.
    Scalar(N),
    /// An array of values each associated to a unique geometry.
    Array(ArrayView1<'a, N>),
    #[cfg(feature = "polars")]
    Column(Column),
}

impl<'a, N, T> From<&'a T> for FieldSource<'a, N>
where
    T: AsRef<[N]> + ?Sized,
{
    fn from(v: &'a T) -> Self {
        Self::Array(ArrayView1::from(v.as_ref()))
    }
}

macro_rules! dispatch {
    ($all_touched:expr, $dedup:expr, $geoms:expr, $ctx:expr, $writer:expr, $idx:expr) => {
        match ($all_touched, $dedup) {
            (true, true) => process::<N, _, AllTouchedCached, _>($geoms, $ctx, $writer, $idx),
            (true, false) => process::<N, _, AllTouched, _>($geoms, $ctx, $writer, $idx),
            (false, _) => process::<N, _, Standard, _>($geoms, $ctx, $writer, $idx),
        }
    };
}

/// Rasterization trait. Attaches to anything that can be viewed as a [`geo::Geometry`] slice.
/// and produces a [`DenseArray`] or a [`SparseArray`].
pub trait Rasterize {
    fn rasterize<A: ArrayBuilder>(&self, ctx: RasterizeContext<A::Dtype>) -> RusterizeResult<A>;
}

impl<T: AsRef<[Geometry<f64>]> + ?Sized> Rasterize for T {
    fn rasterize<A: ArrayBuilder>(&self, ctx: RasterizeContext<A::Dtype>) -> RusterizeResult<A> {
        A::build(self.as_ref(), ctx)
    }
}

/// [`DenseArray`] or [`SparseArray`] creation trait.
pub trait ArrayBuilder: Sized {
    type Dtype: RasterDtype;

    fn build(geoms: &[Geometry<f64>], ctx: RasterizeContext<Self::Dtype>) -> RusterizeResult<Self>;
}

impl<N> ArrayBuilder for DenseArray<N>
where
    N: RasterDtype,
{
    type Dtype = N;

    fn build(geoms: &[Geometry<f64>], ctx: RasterizeContext<Self::Dtype>) -> RusterizeResult<Self> {
        assert_matching_len(geoms.len(), &ctx.field, ctx.by)?;

        let dedup = ctx.requires_dedup();

        match ctx.by {
            Some(by) => {
                let (groups, groups_idx) = group_keys(by);
                let n_groups = groups.len();
                let mut band_names = Vec::with_capacity(n_groups);
                let mut raster = ctx.raster_info.build_raster(n_groups, ctx.background);

                raster
                    .outer_iter_mut()
                    .into_par_iter()
                    .zip(groups.into_par_iter())
                    .zip(groups_idx.into_par_iter())
                    .map(|((band, name), idxs)| {
                        let mut writer = DenseArrayWriter::new(band, ctx.pixel_fn());

                        dispatch!(ctx.all_touched, dedup, geoms, &ctx, &mut writer, idxs.iter().copied());

                        name
                    })
                    .collect_into_vec(&mut band_names);

                Ok(DenseArray::new(raster, band_names, ctx.raster_info))
            }
            None => {
                let band_names = vec![String::from("band_1")];
                let mut raster = ctx.raster_info.build_raster(1, ctx.background);
                let mut writer = DenseArrayWriter::new(raster.index_axis_mut(Axis(0), 0), ctx.pixel_fn());

                dispatch!(ctx.all_touched, dedup, geoms, &ctx, &mut writer, 0..geoms.len());

                Ok(DenseArray::new(raster, band_names, ctx.raster_info))
            }
        }
    }
}

impl<N> ArrayBuilder for SparseArray<N>
where
    N: RasterDtype,
{
    type Dtype = N;

    fn build(geoms: &[Geometry<f64>], ctx: RasterizeContext<Self::Dtype>) -> RusterizeResult<Self> {
        assert_matching_len(geoms.len(), &ctx.field, ctx.by)?;

        let dedup = ctx.requires_dedup();

        match ctx.by {
            Some(by) => {
                let (groups, groups_idx) = group_keys(by);
                let mut writers = Vec::with_capacity(groups.len());

                groups
                    .into_par_iter()
                    .zip(groups_idx.into_par_iter())
                    .map(|(name, idxs)| {
                        let mut writer = SparseArrayWriter::new(name);

                        dispatch!(ctx.all_touched, dedup, geoms, &ctx, &mut writer, idxs.iter().copied());

                        writer
                    })
                    .collect_into_vec(&mut writers);

                Ok(writers.finish(ctx))
            }
            None => {
                let mut writer = SparseArrayWriter::new(String::from("band_1"));

                dispatch!(ctx.all_touched, dedup, geoms, &ctx, &mut writer, 0..geoms.len());

                Ok(writer.finish(ctx))
            }
        }
    }
}

/// Burn the geometries at `indices` onto `writer`.
/// `indices` is `0..len` for a single band, or the group's geometry indexes for multiband.
fn process<N, W, S, I>(geoms: &[Geometry<f64>], ctx: &RasterizeContext<N>, writer: &mut W, indices: I)
where
    N: RasterDtype,
    W: PixelWriter<N>,
    S: LineBurnStrategy,
    I: Iterator<Item = usize>,
{
    match &ctx.field {
        FieldSource::Scalar(s) => {
            for i in indices {
                geoms[i].burn::<S>(&ctx.raster_info, *s, writer, ctx.background);
            }
        }
        FieldSource::Array(arr) => {
            for i in indices {
                geoms[i].burn::<S>(&ctx.raster_info, arr[i], writer, ctx.background);
            }
        }
        #[cfg(feature = "polars")]
        FieldSource::Column(col) => {
            let ca = col.as_materialized_series().unpack::<N::ChunkedArrayType>().unwrap();
            if let Ok(slice) = ca.cont_slice() {
                for i in indices {
                    geoms[i].burn::<S>(&ctx.raster_info, slice[i], writer, ctx.background);
                }
            } else {
                for i in indices {
                    if let Some(fv) = ca.get(i) {
                        geoms[i].burn::<S>(&ctx.raster_info, fv, writer, ctx.background);
                    }
                }
            }
        }
    }
}

/// Group `by` keys into (band name, geometry indexes) pairs, sorted by key.
fn group_keys(by: &[String]) -> (Vec<String>, Vec<Vec<usize>>) {
    let mut groups: HashMap<&String, Vec<usize>> = HashMap::new();
    for (i, key) in by.iter().enumerate() {
        groups.entry(key).or_default().push(i);
    }
    let mut pairs: Vec<(String, Vec<usize>)> = groups.into_iter().map(|(k, idxs)| (k.clone(), idxs)).collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs.into_iter().unzip()
}

/// Validate length of geometry, field, and by. Must match.
fn assert_matching_len<N>(n_geoms: usize, field: &FieldSource<N>, by: Option<&[String]>) -> RusterizeResult<()> {
    let field_len = match field {
        FieldSource::Array(arr) => Some(arr.len()),
        #[cfg(feature = "polars")]
        FieldSource::Column(col) => Some(col.len()),
        FieldSource::Scalar(_) => None,
    };

    if let Some(field_len) = field_len
        && field_len != n_geoms
    {
        return Err(RusterizeError::ValueError("Geometry and field lengths must match"));
    }

    if let Some(by) = by
        && by.len() != n_geoms
    {
        return Err(RusterizeError::ValueError("Geometry and by lengths must match"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geo::raster::RasterInfo, rasterization::pixel_functions::PixelFunction};
    use geo::{Geometry, LineString, Polygon};

    fn raster_4x4() -> RasterInfo {
        RasterInfo {
            ncols: 4,
            nrows: 4,
            xmin: 0.0,
            xmax: 4.0,
            ymin: 0.0,
            ymax: 4.0,
            xres: 1.0,
            yres: 1.0,
            epsg: None,
        }
    }

    #[test]
    fn dense_burns_a_polygon() {
        let poly = Polygon::new(
            LineString::from(vec![(0.5, 0.5), (3.5, 0.5), (3.5, 3.5), (0.5, 3.5), (0.5, 0.5)]),
            vec![],
        );
        let geoms = vec![Geometry::Polygon(poly)];
        let ctx = RasterizeContext {
            raster_info: raster_4x4(),
            field: FieldSource::Scalar(1.0_f64),
            by: None,
            pixel_fn: PixelFunction::Last,
            background: 0.0,
            all_touched: false,
        };

        let out: DenseArray<f64> = geoms.rasterize(ctx).unwrap();
        let (raster, _, _) = out.into_parts();
        assert_eq!(raster.shape(), &[1, 4, 4]);
        assert!(
            raster.iter().any(|&v| v == 1.0),
            "polygon should burn at least one cell"
        );
    }

    #[test]
    fn multiband_burns_only_its_group() {
        use geo::Point;
        use ndarray::Array1;
        let geoms = vec![
            Geometry::Point(Point::new(0.5, 0.5)),
            Geometry::Point(Point::new(3.5, 3.5)),
        ];
        let by = [String::from("a"), String::from("b")];
        let vals = Array1::from(vec![1.0_f64, 2.0]);
        let ctx = RasterizeContext {
            raster_info: raster_4x4(),
            field: FieldSource::Array(vals.view()),
            by: Some(&by[..]),
            pixel_fn: PixelFunction::Last,
            background: 0.0,
            all_touched: false,
        };

        let out: DenseArray<f64> = geoms.rasterize(ctx).unwrap();
        let (raster, _, _) = out.into_parts();
        assert_eq!(raster.shape(), &[2, 4, 4]);

        for band in raster.outer_iter() {
            let has1 = band.iter().any(|&v| v == 1.0);
            let has2 = band.iter().any(|&v| v == 2.0);
            assert!(has1 ^ has2, "a band burned geometries outside its group");
        }
    }

    #[test]
    fn group_keys_groups_and_names() {
        let by = [String::from("b"), String::from("a"), String::from("b")];
        let (names, idx) = group_keys(&by);
        let mut pairs: Vec<(String, Vec<usize>)> = names.into_iter().zip(idx).collect();
        pairs.sort();
        assert_eq!(pairs, vec![("a".to_string(), vec![1]), ("b".to_string(), vec![0, 2])]);
    }
}
