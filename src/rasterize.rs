/*
Rasterize a single (multi)polygon or (multi)linestring.
 */

use crate::edgelist;
use crate::pixel_functions::PixelFn;
use crate::structs::edge::{less_by_x_line, less_by_x_poly, less_by_ystart, Edge};
use crate::structs::{
    edge::{LineEdge, PolyEdge},
    raster::RasterInfo,
};
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
    // build edgelist and sort
    let mut edges: Vec<Edge> = Vec::new();
    edgelist::build_edges(&mut edges, geom, raster_info);

    // early return if no edges
    if edges.is_empty() {
        return;
    }
    edges.par_sort_by(less_by_ystart);

    // start with first y line
    let first_edge = edges.first().unwrap();
    let mut yline = first_edge.ystart();

    // branch by geometry type
    let ncols = raster_info.ncols as f64;
    match first_edge {
        Edge::PolyEdge(_) => {
            // active edges
            let mut active_edges: Vec<PolyEdge> = Vec::new();

            // rasterize loop
            while yline < raster_info.nrows && !(active_edges.is_empty() && edges.is_empty()) {
                // transfer current edges ref to active edges
                active_edges.extend(
                    edges
                        .extract_if(.., |edge| edge.ystart() <= yline) // experimental
                        .filter_map(|edge| PolyEdge::try_from(edge).ok()),
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
        Edge::LineEdge(_) => {
            // extract Edge variant
            let mut active_edges: Vec<LineEdge> = edges
                .into_iter()
                .filter_map(|edge| LineEdge::try_from(edge).ok())
                .collect();
            // sort edges
            active_edges.par_sort_by(less_by_x_line);

            // fill
            for mut edge in active_edges {
                for _ in 0..edge.nsteps {
                    let xstart = edge.x0.clamp(0.0, ncols).ceil() as usize;
                    let xend = edge.x0.clamp(0.0, ncols).ceil() as usize;

                    // fill the pixels between xstart and xend
                    for xpix in xstart..xend {
                        pxfn(ndarray, yline, xpix, field_value, background);
                    }

                    // next move
                    edge.x0 += edge.dx;
                    edge.y0 += edge.dy;
                }
            }
        }
    }
}
