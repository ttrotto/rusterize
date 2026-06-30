/// Optional flags at Python runtime
#[derive(Copy, Clone)]
pub struct OptionalFlags {
    /// Burn all pixels that are touched by the geometry
    pub all_touched: bool,
    /// Output return type is Xarray
    pub xarray: bool,
}

impl OptionalFlags {
    pub fn new(all_touched: bool, encoding: &str) -> Self {
        Self {
            all_touched,
            xarray: encoding == "xarray",
        }
    }
}
