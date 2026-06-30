use thiserror::Error;

#[derive(Error, Debug)]
pub enum RusterizeError {
    #[error("{0}")]
    RuntimeError(&'static str),
    #[error("{0}")]
    ValueError(&'static str),
    #[cfg(feature = "polars")]
    #[error(transparent)]
    Polars(#[from] polars::error::PolarsError),
}

pub type RusterizeResult<T> = std::result::Result<T, RusterizeError>;
