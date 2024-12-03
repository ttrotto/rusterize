/*
Rasterize a single polygon
 */

use crate::edgelist;
use crate::pixel_functions::PixelFn;
use crate::structs::edge::{less_by_x, less_by_ystart};
use crate::structs::{edge::Edge, raster::Raster};
use ndarray::Array2;

pub fn rasterize_polygon(
    raster: &Raster,
    polygon: Vec<f64>,
    poly_value: &f64,
    ndarray: &mut Array2<f64>,
    pxfn: &PixelFn,
) -> () {
    // build edgelist and sort
    let mut edges = edgelist::build_edges(polygon, raster).unwrap();
    edges.sort_by(less_by_ystart);

    // init active edges
    let mut active_edges: Vec<Edge> = Vec::new();

    // start with first y line
    let mut yline = edges.first().unwrap().ystart;

    // init loop objects
    let (mut counter, mut xstart, mut xend): (usize, usize, usize);
    xstart = 0;

    // rasterize loop
    while yline < raster.nrows && !(active_edges.is_empty() && edges.is_empty()) {
        // transfer current edges ref to active edges
        // active_edges.extend(
        //     edges
        //         .iter()
        //         .filter(|edge| edge.ystart <= yline)
        //         .cloned()
        // );
        active_edges.extend(
            edges
                .extract_if(|edge| edge.ystart <= yline) // experimental
                .collect::<Vec<Edge>>(),
        );
        // sort active edges by x
        active_edges.sort_by(less_by_x);

        // even-odd polygon fill
        counter = 0;
        for edge in &active_edges {
            counter += 1;
            let x = if edge.x < 0.0 {
                0.0
            } else if edge.x > raster.ncols as f64 {
                raster.ncols as f64
            } else {
                edge.x
            }
            .ceil() as usize;
            if counter % 2 != 0 {
                xstart = x;
            } else {
                xend = x;
                for xpix in xstart..xend {
                    pxfn(ndarray, yline, xpix, poly_value);
                }
            }
        }
        yline += 1;

        active_edges.retain_mut(|edge| {
            if edge.yend <= yline {
                // drop edges above horizontal line
                false
            } else {
                // update x-position of the next intercepts of edges for the next row
                edge.x += edge.dxdy;
                true
            }
        })
    }
}
