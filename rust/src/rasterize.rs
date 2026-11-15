use crate::{
    encoding::{
        arrays::{DenseArray, SparseArray},
        writers::{DenseArrayWriter, PixelWriter, SparseArrayWriter, finish_sparse},
    },
    error::{RusterizeError, RusterizeResult},
    prelude::{ByGroups, RasterDtype, RasterizeContext},
    rasterization::{
        burn_geometry::{Buffers, Burn},
        burners::{AllTouched, AllTouchedCached, LineBurnStrategy, Standard, StandardCached},
    },
};
use geo::Geometry;
use ndarray::{ArrayView1, Axis};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

#[cfg(feature = "polars")]
use polars::prelude::*;

/// Source of values to burn onto a [`DenseArray`] or [`SparseArray`].
///
/// NaN values (and nulls in a polars array) are skipped: their geometry burns nothing but still counts toward
/// the raster extent when that is derived from the geometries.
#[derive(Clone)]
pub enum FieldSource<'a, N: RasterDtype> {
    /// A single constant value to burn.
    Scalar(N),
    /// An [`ndarray::Array`] of values each associated to a unique geometry.
    Array(ArrayView1<'a, N>),
    /// A typed [`polars::chunked_array::ChunkedArray`] of values each associated to a unique geometry.
    #[cfg(feature = "polars")]
    Polars(ChunkedArray<N::ChunkedArrayType>),
}

impl<'a, N, T> From<&'a T> for FieldSource<'a, N>
where
    N: RasterDtype,
    T: AsRef<[N]> + ?Sized,
{
    fn from(v: &'a T) -> Self {
        Self::Array(ArrayView1::from(v))
    }
}

#[cfg(feature = "polars")]
impl<'a, N: RasterDtype> TryFrom<Series> for FieldSource<'a, N> {
    type Error = RusterizeError;
    fn try_from(v: Series) -> RusterizeResult<Self> {
        Ok(Self::Polars(v.unpack::<N::ChunkedArrayType>()?.clone()))
    }
}

#[cfg(feature = "polars")]
impl<'a, N: RasterDtype> TryFrom<Column> for FieldSource<'a, N> {
    type Error = RusterizeError;
    fn try_from(c: Column) -> RusterizeResult<Self> {
        c.take_materialized_series().try_into()
    }
}

macro_rules! dispatch {
    ($all_touched:expr, $dedup:expr, $geoms:expr, $ctx:expr, $writer:expr, $idx:expr) => {
        match ($all_touched, $dedup) {
            (true, true) => process::<_, _, AllTouchedCached, _>($geoms, $ctx, $writer, $idx),
            (true, false) => process::<_, _, AllTouched, _>($geoms, $ctx, $writer, $idx),
            (false, true) => process::<_, _, StandardCached, _>($geoms, $ctx, $writer, $idx),
            (false, false) => process::<_, _, Standard, _>($geoms, $ctx, $writer, $idx),
        }
    };
}

/// Rasterization trait. Attaches to anything that can be viewed as a [`geo::Geometry`] slice
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
        validate_inputs(geoms.len(), &ctx.field, ctx.by)?;

        let dedup = ctx.requires_dedup();

        match ctx.by {
            Some(by) => {
                let groups = group_codes(by)?;
                let n_groups = groups.len();
                let mut band_names = Vec::with_capacity(n_groups);
                let mut raster = ctx.raster_info.build_raster(n_groups, ctx.background);

                raster
                    .outer_iter_mut()
                    .into_par_iter()
                    .zip(groups.into_par_iter())
                    .map(|(band, (name, idxs))| {
                        let mut writer = DenseArrayWriter::new(band, ctx.pixel_fn);

                        dispatch!(ctx.all_touched, dedup, geoms, &ctx, &mut writer, idxs.iter().copied());

                        name.clone()
                    })
                    .collect_into_vec(&mut band_names);

                Ok(DenseArray::new(raster, band_names, ctx.raster_info))
            }
            None => {
                let band_names = vec![String::from("band_1")];
                let mut raster = ctx.raster_info.build_raster(1, ctx.background);
                let mut writer = DenseArrayWriter::new(raster.index_axis_mut(Axis(0), 0), ctx.pixel_fn);

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
        validate_inputs(geoms.len(), &ctx.field, ctx.by)?;

        let dedup = ctx.requires_dedup();

        match ctx.by {
            Some(by) => {
                let groups = group_codes(by)?;
                let mut writers = Vec::with_capacity(groups.len());

                groups
                    .into_par_iter()
                    .map(|(name, idxs)| {
                        let mut writer = SparseArrayWriter::new(name.clone());

                        dispatch!(ctx.all_touched, dedup, geoms, &ctx, &mut writer, idxs.iter().copied());

                        writer
                    })
                    .collect_into_vec(&mut writers);

                Ok(finish_sparse(writers, ctx))
            }
            None => {
                let mut writer = SparseArrayWriter::new(String::from("band_1"));

                dispatch!(ctx.all_touched, dedup, geoms, &ctx, &mut writer, 0..geoms.len());

                Ok(finish_sparse(vec![writer], ctx))
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
    // reused by every geometry in this band
    let mut buffers = Buffers::default();
    match &ctx.field {
        FieldSource::Scalar(s) => {
            for i in indices {
                geoms[i].burn::<S>(&ctx.raster_info, *s, writer, ctx.background, &mut buffers);
            }
        }
        FieldSource::Array(arr) => {
            for i in indices {
                let field_value = arr[i];
                if !field_value.is_nan() {
                    geoms[i].burn::<S>(&ctx.raster_info, field_value, writer, ctx.background, &mut buffers);
                }
            }
        }
        #[cfg(feature = "polars")]
        FieldSource::Polars(ca) => {
            if let Ok(slice) = ca.cont_slice() {
                for i in indices {
                    let fv = slice[i];
                    if !fv.is_nan() {
                        geoms[i].burn::<S>(&ctx.raster_info, fv, writer, ctx.background, &mut buffers);
                    }
                }
            } else {
                for i in indices {
                    if let Some(fv) = ca.get(i)
                        && !fv.is_nan()
                    {
                        geoms[i].burn::<S>(&ctx.raster_info, fv, writer, ctx.background, &mut buffers);
                    }
                }
            }
        }
    }
}

/// Bucket geometry indexes by group code. Labels without geometries get no band.
fn group_codes(by: ByGroups<'_>) -> RusterizeResult<Vec<(&String, Vec<usize>)>> {
    let mut buckets = vec![Vec::new(); by.labels.len()];
    for (i, &code) in by.codes.iter().enumerate() {
        // callers mark nulls with an out-of-range code
        buckets
            .get_mut(code as usize)
            .ok_or(RusterizeError::ValueError(
                "Found nulls in `by` column. Consider dropping them.",
            ))?
            .push(i);
    }
    Ok(by
        .labels
        .iter()
        .zip(buckets)
        .filter(|(_, idxs)| !idxs.is_empty())
        .collect())
}

/// Validate length of geometry, field, and by. Must match.
fn validate_inputs<N: RasterDtype>(
    n_geoms: usize,
    field: &FieldSource<N>,
    by: Option<ByGroups>,
) -> RusterizeResult<()> {
    let field_len = match field {
        FieldSource::Array(arr) => Some(arr.len()),
        #[cfg(feature = "polars")]
        FieldSource::Polars(ca) => Some(ca.len()),
        FieldSource::Scalar(_) => None,
    };

    if let Some(field_len) = field_len
        && field_len != n_geoms
    {
        return Err(RusterizeError::ValueError("Geometry and field lengths must match"));
    }

    if let Some(by) = by
        && by.codes.len() != n_geoms
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
        let labels = [String::from("a"), String::from("b")];
        let vals = Array1::from(vec![1.0_f64, 2.0]);
        let ctx = RasterizeContext {
            raster_info: raster_4x4(),
            field: FieldSource::Array(vals.view()),
            by: Some(ByGroups {
                codes: &[0, 1],
                labels: &labels,
            }),
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
    fn group_codes_groups_and_names() {
        let labels = [String::from("b"), String::from("a"), String::from("unused")];
        let by = ByGroups {
            codes: &[0, 1, 0],
            labels: &labels,
        };
        assert_eq!(
            group_codes(by).unwrap(),
            vec![(&labels[0], vec![0, 2]), (&labels[1], vec![1])]
        );
        // out-of-range code (a null) is an error
        assert!(
            group_codes(ByGroups {
                codes: &[0, u32::MAX],
                labels: &labels
            })
            .is_err()
        );
    }

    // 10 world units square, with `nrows = 10 / yres` and `ncols = 10 / xres`
    fn raster_10(xres: f64, yres: f64) -> RasterInfo {
        RasterInfo {
            ncols: (10.0 / xres) as usize,
            nrows: (10.0 / yres) as usize,
            xmin: 0.0,
            xmax: 10.0,
            ymin: 0.0,
            ymax: 10.0,
            xres,
            yres,
            epsg: None,
        }
    }

    fn burn<'a>(
        geoms: &[Geometry<f64>],
        raster_info: RasterInfo,
        field: FieldSource<'a, f64>,
        pixel_fn: PixelFunction,
        all_touched: bool,
    ) -> ndarray::Array3<f64> {
        let ctx = RasterizeContext {
            raster_info,
            field,
            by: None,
            pixel_fn,
            background: 0.0,
            all_touched,
        };
        geoms.rasterize::<DenseArray<f64>>(ctx).unwrap().array
    }

    fn self_crossing_line() -> Geometry<f64> {
        Geometry::LineString(LineString::from(vec![(0.5, 0.5), (8.5, 8.5), (8.5, 0.5), (0.5, 8.5)]))
    }

    #[test]
    fn sum_counts_each_pixel_once_per_geometry() {
        // a self-crossing line and the shared vertices between segments must not add twice (GDAL MERGE_ALG=ADD)
        for all_touched in [false, true] {
            for (xres, yres) in [(1.0, 1.0), (1.0, 0.5), (0.5, 1.0)] {
                let r = burn(
                    &[self_crossing_line()],
                    raster_10(xres, yres),
                    FieldSource::Scalar(1.0),
                    PixelFunction::Sum,
                    all_touched,
                );
                let max = r.iter().copied().fold(0.0, f64::max);
                assert_eq!(max, 1.0, "all_touched={all_touched} res=({xres}, {yres})");
            }
        }
    }

    #[test]
    fn dedup_does_not_change_coverage() {
        // `Sum` (deduplicated) and `Last` (not deduplicated) must burn the same pixels
        for all_touched in [false, true] {
            let raster = || raster_10(1.0, 0.5);
            let sum = burn(
                &[self_crossing_line()],
                raster(),
                FieldSource::Scalar(1.0),
                PixelFunction::Sum,
                all_touched,
            );
            let last = burn(
                &[self_crossing_line()],
                raster(),
                FieldSource::Scalar(1.0),
                PixelFunction::Last,
                all_touched,
            );
            assert_eq!(sum, last, "all_touched={all_touched}");
        }
    }

    #[test]
    fn multilinestring_burns_end_pixel_of_every_part() {
        // two disjoint parts, each ending in its own pixel
        let mls = geo::MultiLineString::new(vec![
            LineString::from(vec![(0.5, 9.5), (3.5, 9.5)]),
            LineString::from(vec![(0.5, 5.5), (3.5, 5.5)]),
        ]);
        let r = burn(
            &[Geometry::MultiLineString(mls)],
            raster_10(1.0, 1.0),
            FieldSource::Scalar(1.0),
            PixelFunction::Last,
            false,
        );
        // (x=3.5, y=9.5) -> (row 0, col 3); (x=3.5, y=5.5) -> (row 4, col 3)
        assert_eq!(r[[0, 0, 3]], 1.0, "end pixel of first part");
        assert_eq!(r[[0, 4, 3]], 1.0, "end pixel of second part");
    }

    fn two_overlapping_squares() -> Vec<Geometry<f64>> {
        let square = || {
            Geometry::Polygon(Polygon::new(
                LineString::from(vec![(0.5, 0.5), (3.5, 0.5), (3.5, 3.5), (0.5, 3.5), (0.5, 0.5)]),
                vec![],
            ))
        };
        vec![square(), square()]
    }

    #[test]
    fn nan_array_values_are_skipped() {
        use ndarray::Array1;
        let vals = Array1::from(vec![5.0_f64, f64::NAN]);
        for (name, pixel_fn) in [("last", PixelFunction::Last), ("sum", PixelFunction::Sum)] {
            let r = burn(
                &two_overlapping_squares(),
                raster_4x4(),
                FieldSource::Array(vals.view()),
                pixel_fn,
                false,
            );
            assert!(r.iter().all(|v| !v.is_nan()), "{name} burned a NaN");
            assert!(r.iter().any(|&v| v == 5.0), "{name} dropped the valid value");
        }
    }

    #[cfg(feature = "polars")]
    #[test]
    fn nan_and_null_typed_array_values_are_skipped() {
        // no nulls -> contiguous slice path; with a null -> per-item `get` path
        let contiguous = Float64Chunked::from_slice("v".into(), &[5.0, f64::NAN]);
        let with_null = Float64Chunked::from_slice_options("v".into(), &[Some(5.0), Some(f64::NAN), None]);
        let mut geoms = two_overlapping_squares();
        for (ca, n) in [(contiguous, 2), (with_null, 3)] {
            geoms.resize(n, geoms[0].clone());
            for (name, pixel_fn) in [("last", PixelFunction::Last), ("sum", PixelFunction::Sum)] {
                let r = burn(&geoms, raster_4x4(), FieldSource::Polars(ca.clone()), pixel_fn, false);
                assert!(r.iter().all(|v| !v.is_nan()), "{name} burned a NaN ({n} values)");
                assert!(
                    r.iter().any(|&v| v == 5.0),
                    "{name} dropped the valid value ({n} values)"
                );
            }
        }
    }
}
