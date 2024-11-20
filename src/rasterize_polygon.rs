/*
Rasterize a single polygon
 */

use crate::structs::{edge::Edge, raster::Raster, raster};
use crate::edgelist;
use crate::pixel_functions::PixelFn;
use crate::structs::edge::{less_by_x, less_by_ystart};

pub fn rasterize_polygon(raster: &Raster,
                         polygon: Vec<Vec<f64>>,
                         poly_value: &f64,
                         // array: &NDArray,
                         pxfn: &PixelFn) -> () {
    // build array from raster object
    let mut ndarray = raster::build_2d_array(raster).unwrap();

    // build edgelist and sort
    let mut edges = edgelist::build_edges(polygon, raster).unwrap();
    edges.sort_by(less_by_ystart);

    // init active edges
    let mut active_edges: Vec<Edge> = Vec::new();

    // start with first y line
    let yline = edges.first().unwrap().ystart;

    // init loop objects
    let (mut counter, mut xstart, mut xend, xpix): (usize, usize, usize, usize);
    xstart = 0;

    // rasterize loop
    while yline < raster.nrows &&
        !(active_edges.is_empty() && edges.is_empty()) {
        // transfer current edges ref to active edges
        active_edges.extend(
            edges
                .iter()
                .filter(|edge| edge.ystart <= yline)
                .cloned()
        );
        // sort active edges by x
        active_edges.sort_by(less_by_x);

        // even-odd polygon fill
        counter = 0;
        for it in &active_edges {
            counter += 1;
            let x: usize;
            if it.x < 0.0 {
                x = 0;
            } else if it.x > raster.ncols as f64 {
                x = raster.ncols;
            } else {
                x = it.x.ceil() as usize;
            }
            match counter % 2 {
                0 => xstart = x,
                1 => {
                    xend = x;
                    for xpix in xstart..xend {
                        pxfn(&mut ndarray, yline, xpix, poly_value);
                    }
                },
                _ => unreachable!()
            }
        }
    }
}