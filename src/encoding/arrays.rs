/* Handle array encoding creation and conversion */

use crate::{
    encoding::{
        build_xarray::build_xarray,
        pyarrays::{PyOut, PySparseArray, PySparseArrayTraits, Pythonize},
    },
    geo::raster::RasterInfo,
    prelude::{OptFlags, PolarsHandler},
    rasterization::{pixel_functions::PixelFn, rusterize_impl::RasterizeContext},
};
use ndarray::Array3;
use num_traits::Num;
use numpy::{Element, IntoPyArray};
use polars::prelude::*;
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub struct DenseArray<N> {
    raster: Array3<N>,
    band_names: Vec<String>,
    raster_info: RasterInfo,
}

impl<N: Num> DenseArray<N> {
    pub fn new(raster: Array3<N>, band_names: Vec<String>, raster_info: RasterInfo) -> Self {
        Self {
            raster,
            band_names,
            raster_info,
        }
    }
}

// conversion to python
impl<N> Pythonize for DenseArray<N>
where
    N: Num + Element,
{
    fn pythonize(self, py: Python, opt_flags: OptFlags) -> PyResult<PyOut> {
        let data = self.raster.into_pyarray(py);

        if opt_flags.with_xarray_output() {
            let xarray = build_xarray(py, self.raster_info, data, self.band_names)?;
            Ok(PyOut::Dense(xarray))
        } else {
            Ok(PyOut::Dense(data.into_any()))
        }
    }
}

// triplets of (row, col, value) for all bands as a contiguous block
struct Triplets<N> {
    rows: Vec<usize>,
    cols: Vec<usize>,
    data: Vec<N>,
}

impl<N: Num> Triplets<N> {
    fn new(rows: Vec<usize>, cols: Vec<usize>, data: Vec<N>) -> Self {
        Self { rows, cols, data }
    }
}

pub struct SparseArray<N> {
    band_names: Vec<String>,
    triplets: Triplets<N>,
    lengths: Vec<usize>,
    raster_info: RasterInfo,
    pxfn: PixelFn<N>,
    background: N,
}

impl<N> SparseArray<N>
where
    N: Num + Copy,
{
    pub fn new(
        band_names: Vec<String>,
        rows: Vec<usize>,
        cols: Vec<usize>,
        data: Vec<N>,
        lengths: Vec<usize>,
        ctx: RasterizeContext<N>,
    ) -> Self {
        Self {
            band_names,
            triplets: Triplets::new(rows, cols, data),
            lengths,
            raster_info: ctx.raster_info,
            pxfn: ctx.pixel_fn,
            background: ctx.background,
        }
    }

    fn build_raster(&self) -> Array3<N> {
        let mut raster = self.raster_info.build_raster(self.band_names.len(), self.background);

        let offset = 0;
        let rows = self.triplets.rows.as_slice();
        let cols = self.triplets.cols.as_slice();
        let data = self.triplets.data.as_slice();

        // works with single and multiband rasters
        raster
            .outer_iter_mut()
            .zip(self.lengths.iter())
            .for_each(|(mut band, n)| {
                let end = offset + *n;
                let band_rows = &rows[offset..end];
                let band_cols = &cols[offset..end];
                let band_data = &data[offset..end];

                for ((band_row, band_col), band_value) in band_rows.iter().zip(band_cols).zip(band_data) {
                    (self.pxfn)(&mut band, *band_row, *band_col, *band_value, self.background);
                }
            });
        raster
    }
}

impl<T> PySparseArrayTraits for SparseArray<T>
where
    T: Num + Element + Copy + PolarsHandler,
{
    // estimated size of the materialized array
    fn size_str(&self) -> String {
        let bytesize = size_of_val(&self.background);
        let bytes = bytesize * self.raster_info.nrows * self.raster_info.ncols;

        if bytes < 1000 {
            format!("{} bytes", bytes)
        } else if bytes < 1000 * 1000 {
            format!("{:.2} KB", bytes as f32 / 1000.0)
        } else if bytes < 1000 * 1000 * 1000 {
            format!("{:.2} MB", bytes as f32 / (1000.0 * 1000.0))
        } else {
            format!("{:.2} GB", bytes as f32 / (1000.0 * 1000.0 * 1000.0))
        }
    }

    fn extent(&self) -> (&f64, &f64, &f64, &f64) {
        (
            &self.raster_info.xmin,
            &self.raster_info.ymin,
            &self.raster_info.xmax,
            &self.raster_info.ymax,
        )
    }

    fn shape(&self) -> (&usize, &usize) {
        (&self.raster_info.nrows, &self.raster_info.ncols)
    }

    fn resolution(&self) -> (&f64, &f64) {
        (&self.raster_info.yres, &self.raster_info.yres)
    }

    fn epsg(&self) -> &Option<u16> {
        &self.raster_info.epsg
    }

    fn to_xarray<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let raster = self.build_raster();

        let data = raster.into_pyarray(py);

        build_xarray(py, self.raster_info.clone(), data, self.band_names.clone())
    }

    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let raster = self.build_raster();
        Ok(raster.into_pyarray(py).into_any())
    }

    fn to_frame(&self) -> PyDataFrame {
        let mut columns = Vec::new();

        // add bands for multiband raster
        if self.lengths.len() > 1 {
            let bands = self
                .lengths
                .iter()
                .enumerate()
                .flat_map(|(i, v)| std::iter::repeat_n(i + 1, *v))
                .map(|b| b as u64)
                .collect::<Vec<u64>>();
            let bands_column = Column::new("band".into(), bands);
            columns.push(bands_column);
        }

        let rows = self.triplets.rows.par_iter().map(|v| *v as u64).collect::<Vec<u64>>();
        let length = rows.len();
        columns.push(Column::new("row".into(), rows));

        let cols = self.triplets.cols.par_iter().map(|v| *v as u64).collect::<Vec<u64>>();
        columns.push(Column::new("col".into(), cols));

        columns.push(T::from_named_vec("data", &self.triplets.data));

        let df = DataFrame::new(length, columns).unwrap();
        PyDataFrame(df)
    }
}

// conversion to python
impl<T> Pythonize for SparseArray<T>
where
    T: Num + Element + Copy + PolarsHandler + 'static,
{
    fn pythonize(self, _py: Python, _opt_flags: OptFlags) -> PyResult<PyOut> {
        Ok(PyOut::Sparse(PySparseArray(Arc::new(self))))
    }
}
