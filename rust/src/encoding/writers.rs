use crate::{
    encoding::arrays::SparseArray,
    prelude::{RasterDtype, RasterizeContext},
    rasterization::{pixel_cache::PixelCache, pixel_functions::PixelFunction},
};
use ndarray::ArrayViewMut2;
use num_traits::Num;

/// Trait in charge of writing a pixel onto a [`DenseArray`](crate::DenseArray) or [`SparseArray`].
pub(crate) trait PixelWriter<N: Num> {
    fn write(&mut self, y: usize, x: usize, value: N, background: N);
}

/// Writer that records each pixel and skips repeats within a geometry, used when the pixel function needs
/// dedup: lines with any strategy, and polygon rings with `all_touched` (pass 1).
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

/// Writer for the interior of a polygon with `all_touched` when dedup is required (pass 2): skips the pixels
/// already burned by its rings in pass 1.
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

/// Writer for a [`DenseArray`](crate::DenseArray).
pub struct DenseArrayWriter<'a, N> {
    band: ArrayViewMut2<'a, N>,
    pxfn: PixelFunction,
}

impl<'a, N: RasterDtype> PixelWriter<N> for DenseArrayWriter<'a, N> {
    #[inline(always)]
    fn write(&mut self, y: usize, x: usize, value: N, background: N) {
        self.pxfn.apply(&mut self.band, y, x, value, background);
    }
}

impl<'a, N: Num> DenseArrayWriter<'a, N> {
    pub fn new(band: ArrayViewMut2<'a, N>, pxfn: PixelFunction) -> Self {
        Self { band, pxfn }
    }
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

/// Merge per-band writers into one [`SparseArray`], bands in order.
pub(crate) fn finish_sparse<N: RasterDtype>(
    writers: Vec<SparseArrayWriter<N>>,
    ctx: RasterizeContext<N>,
) -> SparseArray<N> {
    let mut band_names = Vec::with_capacity(writers.len());
    let mut offsets = Vec::with_capacity(writers.len());
    let (mut rows, mut cols, mut data) = (Vec::new(), Vec::new(), Vec::new());
    for writer in writers {
        offsets.push(writer.values.len());
        band_names.push(writer.band_name);
        if rows.is_empty() {
            // take the first non-empty band's buffers instead of copying them
            (rows, cols, data) = (writer.rows, writer.cols, writer.values);
        } else {
            rows.extend(writer.rows);
            cols.extend(writer.cols);
            data.extend(writer.values);
        }
    }

    SparseArray::new(band_names, rows, cols, data, offsets, ctx)
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
