/* Handle array encoding creation and conversion */

use crate::{
    encoding::{
        build_xarray::build_xarray,
        pyarrays::{PyOut, PySparseArray, PySparseArrayTraits, Pythonize},
    },
    geo::raster::RasterInfo,
    prelude::PolarsHandler,
    rasterization::{pixel_functions::PixelFn, rusterize_impl::RasterizeConfig},
};
use ndarray::Array3;
use num_traits::Num;
use numpy::Element;
use polars::prelude::*;
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;

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
    fn pythonize(self, py: Python) -> PyResult<PyOut> {
        let xarray = build_xarray(py, self.raster_info, self.raster, self.band_names)?;
        Ok(PyOut::Dense(xarray))
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

    fn iter(&self) -> impl Iterator<Item = ((&usize, &usize), &N)> {
        self.rows.iter().zip(self.cols.iter()).zip(self.data.iter())
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

impl<N: Num> SparseArray<N> {
    pub fn new(
        band_names: Vec<String>,
        rows: Vec<usize>,
        cols: Vec<usize>,
        data: Vec<N>,
        lengths: Vec<usize>,
        config: RasterizeConfig<N>,
    ) -> Self {
        Self {
            band_names,
            triplets: Triplets::new(rows, cols, data),
            lengths,
            raster_info: config.raster_info,
            pxfn: config.pixel_fn,
            background: config.background,
        }
    }
}

impl<N> PySparseArrayTraits for SparseArray<N>
where
    N: Num + Element + Copy + PolarsHandler,
{
    fn size_str(&self) -> String {
        let bytesize = size_of_val(&self.background);
        let bytes = bytesize * self.raster_info.nrows * self.raster_info.ncols;

        if bytes < 1024 {
            format!("{} bytes", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.2} KB", bytes as f32 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.2} MB", bytes as f32 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f32 / (1024.0 * 1024.0 * 1024.0))
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

    fn epsg(&self) -> &u16 {
        &self.raster_info.epsg
    }

    fn to_xarray<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut raster = self
            .raster_info
            .build_raster(self.band_names.len(), self.background);

        let mut offset = 0;

        // works with single and multiband rasters
        raster
            .outer_iter_mut()
            .zip(self.lengths.iter())
            .for_each(|(mut band, n)| {
                // `skip` jumps to the beginning of the next band and takes `n` pixels
                for ((row, col), value) in self.triplets.iter().skip(offset).take(*n) {
                    (self.pxfn)(&mut band, *row, *col, *value, self.background);
                }
                offset += *n
            });

        build_xarray(
            py,
            self.raster_info.clone(),
            raster,
            self.band_names.clone(),
        )
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
                .map(|b| b as u32)
                .collect::<Vec<u32>>();
            let bands_column = Column::new("band".into(), bands);
            columns.push(bands_column);
        }

        let rows = self
            .triplets
            .rows
            .iter()
            .map(|v| *v as u32)
            .collect::<Vec<u32>>();
        columns.push(Column::new("row".into(), rows));

        let cols = self
            .triplets
            .cols
            .iter()
            .map(|v| *v as u32)
            .collect::<Vec<u32>>();
        columns.push(Column::new("col".into(), cols));

        columns.push(N::from_named_vec("data", &self.triplets.data));

        let df = DataFrame::new(columns).unwrap();
        PyDataFrame(df)
    }
}

// conversion to python
impl<N> Pythonize for SparseArray<N>
where
    N: Num + Element + Copy + PolarsHandler + 'static,
{
    fn pythonize(self, _py: Python) -> PyResult<PyOut> {
        Ok(PyOut::Sparse(PySparseArray(Arc::new(self))))
    }
}
