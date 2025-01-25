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
    background: &f64,
) {
    // build edgelist and sort
    let mut edges: Vec<Edge> = Vec::new();
    edgelist::build_edges(&mut edges, polygon, raster_info);

    // early return if no edges
    if edges.is_empty() {
        return;
    }
    edges.sort_by(less_by_ystart);

    // active edges
    let mut active_edges: Vec<Edge> = Vec::new();

    // start with first y line
    let mut yline = edges.first().unwrap().ystart;

    // rasterize loop
    while yline < raster_info.nrows && !(active_edges.is_empty() && edges.is_empty()) {
        // transfer current edges ref to active edges
        active_edges.extend(
            edges
                .extract_if(.., |edge| edge.ystart <= yline) // experimental
        );
        // sort active edges
        active_edges.sort_by(less_by_x);

        // even-odd polygon fill
        for (edge1, edge2) in active_edges.iter().zip(active_edges.iter().skip(1)) {
            let xstart = edge1.x.clamp(0.0, raster_info.ncols as f64).ceil() as usize;
            let xend = edge2.x.clamp(0.0, raster_info.ncols as f64).ceil() as usize;
            for xpix in xstart..xend {
                pxfn(ndarray, yline, xpix, field_value, background);
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
