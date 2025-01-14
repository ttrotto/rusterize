/*
Rasterize a single (multi)polygon.
 */

use crate::edgelist;
use crate::pixel_functions::PixelFn;
use crate::structs::edge::{less_by_x, less_by_ystart};
use crate::structs::{edge::Edge, raster::RasterInfo};
use geo_types::Geometry;
use numpy::ndarray::ArrayViewMut2;

pub fn rasterize_polygon(
    raster_info: &RasterInfo,
    polygon: &Geometry,
    field_value: &f64,
    ndarray: &mut ArrayViewMut2<f64>,
    pxfn: &PixelFn,
) {
    // build edgelist and sort
    let mut edges: Vec<Edge> = Vec::new();
    edgelist::build_edges(&mut edges, polygon, raster_info);
    edges.sort_by(less_by_ystart);

    // init active edges
    let mut active_edges: Vec<Edge> = Vec::new();

    // start with first y line
    let mut yline = match edges.first() {
        Some(e) => e.ystart,
        None => return, // handle case when no edge to rasterize
    };

    // ranges for x coordinate
    let (mut xstart, mut counter): (usize, usize) = (0, 0);

    // rasterize loop
    while yline < raster_info.nrows && !(active_edges.is_empty() && edges.is_empty()) {
        // transfer current edges ref to active edges
        active_edges.extend(
            edges
                .extract_if(|edge| edge.ystart <= yline) // experimental
                .collect::<Vec<Edge>>(),
        );
        // sort active edges by x
        active_edges.sort_by(less_by_x);

        // even-odd polygon fill
        for edge in &active_edges {
            counter += 1;
            let x = if edge.x < 0.0 {
                0.0
            } else if edge.x > raster_info.ncols as f64 {
                raster_info.ncols as f64
            } else {
                edge.x.ceil()
            } as usize;
            if counter % 2 != 0 {
                xstart = x;
            } else {
                let xend = x;
                for xpix in xstart..xend {
                    pxfn(ndarray, yline, xpix, field_value);
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
