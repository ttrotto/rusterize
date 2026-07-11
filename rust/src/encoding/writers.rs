use crate::{
    encoding::arrays::SparseArray,
    prelude::{RasterDtype, RasterizeContext},
    rasterization::{pixel_cache::PixelCache, pixel_functions::PixelFn},
};
use ndarray::ArrayViewMut2;
use num_traits::Num;

/// Trait in charge of writing a pixel onto a [`DenseArray`] or [`SparseArray`].
pub(crate) trait PixelWriter<N: Num> {
    fn write(&mut self, y: usize, x: usize, value: N, background: N);
}

/// Writer for interior and exterior [`geo::Linestring`] when `all_touched` is true (pass 1).
pub struct LineWriter<'a, W> {
    inner: &'a mut W,
    cache: &'a mut PixelCache,
}

impl<'a, W, N> PixelWriter<N> for LineWriter<'a, W>
where
    N: Num,
    W: PixelWriter<N>,
{
    fn write(&mut self, y: usize, x: usize, value: N, background: N) {
        if self.cache.insert(x, y) {
            self.inner.write(y, x, value, background);
        }
    }
}

impl<'a, W> LineWriter<'a, W> {
    pub(crate) fn new(inner: &'a mut W, cache: &'a mut PixelCache) -> Self {
        Self { inner, cache }
    }
}

/// Writer for filling pixels after burning a [`geo::Linestring`] when `all_touched` is true (pass 2).
pub struct FillWriter<'a, W> {
    inner: &'a mut W,
    cache: &'a mut PixelCache,
}

impl<'a, W, N> PixelWriter<N> for FillWriter<'a, W>
where
    N: Num,
    W: PixelWriter<N>,
{
    fn write(&mut self, y: usize, x: usize, value: N, background: N) {
        if !self.cache.contains(x, y) {
            self.inner.write(y, x, value, background);
        }
    }
}

impl<'a, W> FillWriter<'a, W> {
    pub(crate) fn new(inner: &'a mut W, cache: &'a mut PixelCache) -> Self {
        Self { inner, cache }
    }
}

/// Writer for a [`DenseArray`].
pub struct DenseArrayWriter<'a, N> {
    band: ArrayViewMut2<'a, N>,
    pxfn: PixelFn<N>,
}

impl<'a, N: Num> PixelWriter<N> for DenseArrayWriter<'a, N> {
    fn write(&mut self, y: usize, x: usize, value: N, background: N) {
        (self.pxfn)(&mut self.band, y, x, value, background);
    }
}

impl<'a, N: Num> DenseArrayWriter<'a, N> {
    pub fn new(band: ArrayViewMut2<'a, N>, pxfn: PixelFn<N>) -> Self {
        Self { band, pxfn }
    }
}

/// Convert a [`SparseArrayWriter`] into a [`SparseArray`].
pub trait ToSparseArray<N: Num> {
    fn finish(self, ctx: RasterizeContext<N>) -> SparseArray<N>;
}

/// Writer for a [`SparseArray`].
pub struct SparseArrayWriter<N> {
    pub band_name: String,
    pub rows: Vec<u64>,
    pub cols: Vec<u64>,
    pub values: Vec<N>,
}

impl<N: Num> PixelWriter<N> for SparseArrayWriter<N> {
    fn write(&mut self, y: usize, x: usize, value: N, _background: N) {
        self.rows.push(y as u64);
        self.cols.push(x as u64);
        self.values.push(value);
    }
}

impl<N> ToSparseArray<N> for SparseArrayWriter<N>
where
    N: RasterDtype,
{
    fn finish(self, ctx: RasterizeContext<N>) -> SparseArray<N> {
        let offsets = vec![self.values.len()];
        let band_names = vec![self.band_name];
        SparseArray::new(band_names, self.rows, self.cols, self.values, offsets, ctx)
    }
}

impl<N> ToSparseArray<N> for Vec<SparseArrayWriter<N>>
where
    N: RasterDtype,
{
    fn finish(self, ctx: RasterizeContext<N>) -> SparseArray<N> {
        let (band_names, rows, cols, data, offsets) = self.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |(mut band_names, mut rows, mut cols, mut data, mut offsets), writer| {
                offsets.push(writer.values.len());
                band_names.push(writer.band_name);
                rows.extend(writer.rows);
                cols.extend(writer.cols);
                data.extend(writer.values);
                (band_names, rows, cols, data, offsets)
            },
        );

        SparseArray::new(band_names, rows, cols, data, offsets, ctx)
    }
}

impl<N: Num> SparseArrayWriter<N> {
    pub fn new(band_name: String) -> Self {
        Self {
            band_name,
            rows: Vec::new(),
            cols: Vec::new(),
            values: Vec::new(),
        }
    }
}
