use ndarray::Array3;
use rusterize::{DenseArray, RasterDtype, SparseArray, prelude::RasterInfo};
use savvy::{NotAvailableValue, OwnedIntegerSexp, OwnedRealSexp, OwnedStringSexp, Sexp, savvy, savvy_err};

/// Convert the underlying array in [`rusterize::ArrayBuilder`] to a [`savvy::Sexp`] object.
fn array_as_sexp<N: ArrayAsSexp + RasterDtype>(array: &Array3<N>) -> savvy::Result<Sexp> {
    let (nbands, nrows, ncols) = array.dim();
    let dim = vec![ncols as i32, nrows as i32, nbands as i32];

    let slice = array.as_slice().ok_or(savvy_err!(r"No array found ¯\_(ツ)_/¯."))?;
    N::slice_as_sexp(slice, &dim)
}

/// Corresponding trait for [`array_as_sexp`] to handle variable dtype.
trait ArrayAsSexp: Sized {
    fn slice_as_sexp(s: &[Self], dim: &[i32]) -> savvy::Result<Sexp>;
}

impl ArrayAsSexp for i32 {
    fn slice_as_sexp(s: &[Self], dim: &[i32]) -> savvy::Result<Sexp> {
        let mut out = OwnedIntegerSexp::try_from_slice(s)?;
        out.set_dim(dim)?;
        out.into()
    }
}

impl ArrayAsSexp for f64 {
    fn slice_as_sexp(s: &[Self], dim: &[i32]) -> savvy::Result<Sexp> {
        let mut out = OwnedRealSexp::try_from_slice(s)?;
        out.set_dim(dim)?;
        out.into()
    }
}

/// Methods to access the [`RArray`] passed to R.
pub trait RArrayTraits {
    fn to_raster(&self) -> savvy::Result<Sexp>;
    fn names(&self) -> &[String];
    fn info(&self) -> &RasterInfo;
}

impl<N: ArrayAsSexp + RasterDtype> RArrayTraits for DenseArray<N> {
    fn to_raster(&self) -> savvy::Result<Sexp> {
        array_as_sexp(self.array())
    }

    fn names(&self) -> &[String] {
        self.band_names()
    }

    fn info(&self) -> &RasterInfo {
        self.raster_info()
    }
}

impl<N: ArrayAsSexp + RasterDtype> RArrayTraits for SparseArray<N> {
    fn to_raster(&self) -> savvy::Result<Sexp> {
        array_as_sexp(&self.build_array())
    }

    fn names(&self) -> &[String] {
        self.band_names()
    }

    fn info(&self) -> &RasterInfo {
        self.raster_info()
    }
}

#[savvy]
pub struct RArray(pub Box<dyn RArrayTraits>);

#[savvy]
impl RArray {
    fn to_raster(&self) -> savvy::Result<Sexp> {
        self.0.to_raster()
    }

    fn names(&self) -> savvy::Result<Sexp> {
        OwnedStringSexp::try_from_slice(self.0.names())?.into()
    }

    fn extent(&self) -> savvy::Result<Sexp> {
        let i = self.0.info();
        OwnedRealSexp::try_from_slice([i.xmin, i.ymin, i.xmax, i.ymax])?.into()
    }

    fn resolution(&self) -> savvy::Result<Sexp> {
        let i = self.0.info();
        OwnedRealSexp::try_from_slice([i.xres, i.yres])?.into()
    }

    fn epsg(&self) -> savvy::Result<Sexp> {
        let code = self.0.info().epsg.map(|e| e as i32).unwrap_or(i32::na());
        OwnedIntegerSexp::try_from_scalar(code)?.into()
    }
}
