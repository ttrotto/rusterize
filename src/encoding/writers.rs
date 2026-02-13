/* Handle how pixels are recorded depending on the output format */

use crate::{
    encoding::arrays::SparseArray,
    rasterization::{
        pixel_functions::PixelFn,
        rusterize_impl::{PixelCache, RasterizeContext},
    },
};
use ndarray::ArrayViewMut2;
use num_traits::Num;

pub trait PixelWriter<N: Num> {
    fn write(&mut self, y: usize, x: usize, value: N, background: N);
}

// writer for interior and exterior lines when `all_touched` is true (pass 1)
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
    pub fn new(inner: &'a mut W, cache: &'a mut PixelCache) -> Self {
        Self { inner, cache }
    }
}

// writer for filling pixels after burning lines when `all_touched` is true (pass 2)
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
    pub fn new(inner: &'a mut W, cache: &'a mut PixelCache) -> Self {
        Self { inner, cache }
    }
}

// writer for dense output (numpy/xarray)
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

// convert sparse writer into a sparse array
pub trait ToSparseArray<N: Num> {
    fn finish(self, ctx: RasterizeContext<N>) -> SparseArray<N>;
}

// writer for sparse output (COOrdinate format)
pub struct SparseArrayWriter<N> {
    pub band_name: String,
    pub rows: Vec<usize>,
    pub cols: Vec<usize>,
    pub values: Vec<N>,
}

impl<N: Num> PixelWriter<N> for SparseArrayWriter<N> {
    fn write(&mut self, y: usize, x: usize, value: N, _background: N) {
        self.rows.push(y);
        self.cols.push(x);
        self.values.push(value);
    }
}

impl<N> ToSparseArray<N> for SparseArrayWriter<N>
where
    N: Num + Copy,
{
    fn finish(self, ctx: RasterizeContext<N>) -> SparseArray<N> {
        let lengths = vec![self.values.len()];
        let band_names = vec![self.band_name];
        SparseArray::new(band_names, self.rows, self.cols, self.values, lengths, ctx)
    }
}

impl<N> ToSparseArray<N> for Vec<SparseArrayWriter<N>>
where
    N: Num + Copy,
{
    fn finish(self, ctx: RasterizeContext<N>) -> SparseArray<N> {
        let (band_names, rows, cols, data, lengths) = self.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |(mut band_names, mut rows, mut cols, mut data, mut lengths), writer| {
                lengths.push(writer.values.len());
                band_names.push(writer.band_name);
                rows.extend(writer.rows);
                cols.extend(writer.cols);
                data.extend(writer.values);
                (band_names, rows, cols, data, lengths)
            },
        );

        SparseArray::new(band_names, rows, cols, data, lengths, ctx)
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
