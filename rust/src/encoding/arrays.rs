use crate::{
    geo::raster::RasterInfo,
    prelude::{RasterDtype, RasterizeContext},
    rasterization::pixel_functions::PixelFn,
};
use ndarray::Array3;
use num_traits::Num;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

/// A materialized 3-dimensional array containing the burned geometries and spatial information.
pub struct DenseArray<N> {
    raster: Array3<N>,
    band_names: Vec<String>,
    raster_info: RasterInfo,
}

impl<N: Num> DenseArray<N> {
    pub(crate) fn new(raster: Array3<N>, band_names: Vec<String>, raster_info: RasterInfo) -> Self {
        Self {
            raster,
            band_names,
            raster_info,
        }
    }

    /// Consume self and extract all fields of the DenseArray.
    pub fn into_parts(self) -> (Array3<N>, Vec<String>, RasterInfo) {
        (self.raster, self.band_names, self.raster_info)
    }

    pub fn raster(&self) -> &Array3<N> {
        &self.raster
    }

    /// Sorted band names for the array. Defaults to "band_1" for a single band.
    pub fn band_names(&self) -> &[String] {
        &self.band_names
    }

    /// Spatial information associated with the array.
    pub fn raster_info(&self) -> &RasterInfo {
        &self.raster_info
    }
}

/// Triplets of (row, col, value) for all bands as a contiguous block.
/// Used to store inside a [`SparseArray`].
struct Triplets<N> {
    rows: Vec<u64>,
    cols: Vec<u64>,
    data: Vec<N>,
}

impl<N: Num> Triplets<N> {
    fn new(rows: Vec<u64>, cols: Vec<u64>, data: Vec<N>) -> Self {
        Self { rows, cols, data }
    }
}

/// A sparse array in COOordinate format storing the band/row/col value triplets.
/// of all burned [`geo::Geometry`].
pub struct SparseArray<N> {
    band_names: Vec<String>,
    triplets: Triplets<N>,
    offsets: Vec<usize>,
    raster_info: RasterInfo,
    pxfn: PixelFn<N>,
    background: N,
}

impl<N> SparseArray<N>
where
    N: RasterDtype,
{
    pub(crate) fn new(
        band_names: Vec<String>,
        rows: Vec<u64>,
        cols: Vec<u64>,
        data: Vec<N>,
        offsets: Vec<usize>,
        ctx: RasterizeContext<N>,
    ) -> Self {
        let pxfn = ctx.pixel_fn();
        let background = ctx.background;

        Self {
            band_names,
            triplets: Triplets::new(rows, cols, data),
            offsets,
            raster_info: ctx.raster_info,
            pxfn,
            background,
        }
    }

    /// Get the band names associated with this array.
    pub fn band_names(&self) -> &[String] {
        &self.band_names
    }

    /// Materialize a [`ndarray::Array3`] from this. Drops spatial information.
    pub fn build_array(&self) -> Array3<N> {
        let mut raster = self.raster_info.build_raster(self.band_names.len(), self.background);

        let rows = self.triplets.rows.as_slice();
        let cols = self.triplets.cols.as_slice();
        let data = self.triplets.data.as_slice();

        // per-band start offset into the contiguous triplet arrays
        let offsets = self
            .offsets
            .iter()
            .scan(0, |state, &n| {
                let start = *state;
                *state += n;
                Some(start)
            })
            .collect::<Vec<usize>>();

        raster
            .outer_iter_mut()
            .into_par_iter()
            .zip(self.offsets.par_iter())
            .zip(offsets.par_iter())
            .for_each(|((mut band, n), &off)| {
                let end = off + *n;
                let band_rows = &rows[off..end];
                let band_cols = &cols[off..end];
                let band_data = &data[off..end];

                for ((band_row, band_col), band_value) in band_rows.iter().zip(band_cols).zip(band_data) {
                    (self.pxfn)(
                        &mut band,
                        *band_row as usize,
                        *band_col as usize,
                        *band_value,
                        self.background,
                    );
                }
            });
        raster
    }

    pub fn extent(&self) -> (f64, f64, f64, f64) {
        (
            self.raster_info.xmin,
            self.raster_info.ymin,
            self.raster_info.xmax,
            self.raster_info.ymax,
        )
    }

    pub fn shape(&self) -> (usize, usize, usize) {
        (self.band_names.len(), self.raster_info.nrows, self.raster_info.ncols)
    }

    pub fn resolution(&self) -> (f64, f64) {
        (self.raster_info.xres, self.raster_info.yres)
    }

    /// Get spatial information associated with this array.
    pub fn raster_info(&self) -> &RasterInfo {
        &self.raster_info
    }

    pub fn epsg(&self) -> Option<u16> {
        self.raster_info.epsg
    }
}

#[cfg(feature = "polars")]
mod feature_gated {
    use super::SparseArray;
    use crate::prelude::PolarsHandler;
    use num_traits::Num;
    use polars::prelude::*;

    impl<N> SparseArray<N>
    where
        N: Num + Copy + PolarsHandler,
    {
        /// Convert this to a [`polars::prelude::DataFrame`].
        pub fn to_frame(&self) -> DataFrame {
            let mut columns: Vec<Column> = Vec::new();

            // add bands for multiband raster
            if self.offsets.len() > 1 {
                let bands = self
                    .offsets
                    .iter()
                    .enumerate()
                    .flat_map(|(i, v)| std::iter::repeat_n(i + 1, *v))
                    .map(|b| b as u64)
                    .collect::<Vec<u64>>();
                let bands_column = Column::new("band".into(), bands);
                columns.push(bands_column);
            }

            columns.push(Column::new("row".into(), self.triplets.rows.as_slice()));
            columns.push(Column::new("col".into(), self.triplets.cols.as_slice()));

            let height = self.triplets.data.len();
            columns.push(N::from_named_vec("values", &self.triplets.data));

            DataFrame::new(height, columns).unwrap()
        }
    }
}
