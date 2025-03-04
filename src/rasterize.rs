/*
Rasterize a single (multi)polygon or (multi)linestring.
 */

use crate::edge_collection;
use crate::pixel_functions::PixelFn;
use crate::structs::edge::{
    less_by_x_line, less_by_x_poly, less_by_ystart_line, less_by_ystart_poly, EdgeCollection,
};
use crate::structs::{edge::PolyEdge, raster::RasterInfo};
use edge_collection::build_edges;
use geo_types::Geometry;
use numpy::ndarray::ArrayViewMut2;
use rayon::prelude::*;

pub fn rasterize(
    raster_info: &RasterInfo,
    geom: &Geometry,
    field_value: &f64,
    ndarray: &mut ArrayViewMut2<f64>,
    pxfn: &PixelFn,
    background: &f64,
) {
    // build edge collection
    let edges = build_edges(geom, raster_info);

    // early return if no edges
    if edges.is_empty() {
        return;
    }

    // branch by geometry type
    let ncols = raster_info.ncols as f64;
    let nrows = raster_info.nrows as f64;
    match edges {
        EdgeCollection::PolyEdges(mut polyedges) => {
            // sort edges
            polyedges.par_sort_by(less_by_ystart_poly);

            // start with first y line
            let mut yline = polyedges.first().unwrap().ystart;

            // active edges
            let mut active_edges: Vec<PolyEdge> = Vec::new();

            // rasterize loop
            while yline < raster_info.nrows && !(active_edges.is_empty() && polyedges.is_empty()) {
                // transfer current edges ref to active edges
                active_edges.extend(
                    polyedges.extract_if(.., |edge| edge.ystart <= yline), // experimental
                );
                // sort active edges
                active_edges.par_sort_by(less_by_x_poly);

                // even-odd polygon fill{
                for (edge1, edge2) in active_edges
                    .iter()
                    .zip(active_edges.iter().skip(1))
                    .step_by(2)
                {
                    // clamp and round the x-coordinates of the edges
                    let xstart = edge1.x.clamp(0.0, ncols).ceil() as usize;
                    let xend = edge2.x.clamp(0.0, ncols).ceil() as usize;

                    // fill the pixels between xstart and xend
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
        EdgeCollection::LineEdges(mut linedges) => {
            // sort edges
            linedges.par_sort_by(less_by_ystart_line);
            linedges.par_sort_by(less_by_x_line);

            // fill
            for mut edge in linedges {
                (0..edge.nsteps).for_each(|_| {
                    let x = edge.x0.clamp(0.0, ncols).ceil() as usize;
                    let y = edge.y0.clamp(0.0, nrows).ceil() as usize;

                    // fill the pixels at x-y location
                    pxfn(ndarray, y, x, field_value, background);

                    // next move
                    edge.x0 += edge.dx;
                    edge.y0 += edge.dy;
                })
            }
        }
    }
}
