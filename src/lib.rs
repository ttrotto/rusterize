extern crate blas_src;

mod structs {
    pub mod edge;
    pub mod raster;
}

mod check_inputs;
mod rasterize_polygon;
mod pixel_functions;
mod edgelist;
