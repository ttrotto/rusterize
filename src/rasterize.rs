/*
Rasterize a single (multi)polygon or (multi)linestring.
 */

use crate::edge_collection;
use crate::pixel_functions::PixelFn;
// use crate::structs::edge::{less_by_x_line, less_by_x_poly, less_by_ystart_line, less_by_ystart_poly, EdgeCollection, LineEdge};
use crate::structs::edge::{less_by_x_poly, less_by_ystart_poly, EdgeCollection};
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
                // transfer current edges to active edges
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
            // gdal - simplified
            for mut edge in linedges {
                loop {
                    // Fill the pixel at (x0, y0) if it's within bounds
                    if edge.ix0 >= 0 && edge.ix0 < raster_info.ncols as isize &&
                        edge.iy0 >= 0 && edge.iy0 < raster_info.nrows as isize {
                        if !(edge.ix0 == edge.ix1 && edge.iy0 == edge.iy1 && !edge.is_last_segment) {
                            pxfn(ndarray, edge.iy0 as usize, edge.ix0 as usize, field_value, background);
                        }
                    }

                    // Check if we've reached the end of the line
                    if edge.ix0 == edge.ix1 && edge.iy0 == edge.iy1 {
                        break;
                    }

                    // Update the error term and coordinates
                    let e2 = 2 * edge.err;
                    if e2 >= edge.dy {
                        edge.err += edge.dy;
                        edge.ix0 += edge.sx;
                    }
                    if e2 <= edge.dx {
                        edge.err += edge.dx;
                        edge.iy0 += edge.sy;
                    }
                }
            }
            
            
            // // grid walking
            // for mut edge in linedges {
            //     let (mut ix, mut iy): (usize, usize) = (0, 0);
            //     while ix < edge.nx || iy < edge.ny {
            //         if (1 + 2*ix) * edge.ny < (1 + 2*iy) * edge.nx {
            //             edge.x0 += edge.sign_x;
            //             ix += 1
            //         } else {
            //             edge.y0 += edge.sign_y;
            //             iy += 1
            //         }
            //         let y = (raster_info.ymax - edge.y0) / raster_info.yres - 1.0;
            //         let x = (edge.x0 - raster_info.xmin) / raster_info.xres - 1.0;
            //         pxfn(ndarray, y.round() as usize, x.round() as usize, field_value, background);
            //     }
            // }
            // // sort edges
            // linedges.par_sort_by(less_by_ystart_line);
            // linedges.par_sort_by(less_by_x_line);
            //
            // // fill
            // for mut edge in linedges {
            //     (0..edge.nsteps).for_each(|_| {
            //         // clamp values and adjust to 0-index
            //         let x = edge.x0.clamp(0.0, ncols - 1.0).ceil() as usize;
            //         let y = edge.y0.clamp(0.0, nrows - 1.0).ceil() as usize;
            //
            //         // fill the pixels at x-y location
            //         pxfn(ndarray, y, x, field_value, background);
            //
            //         // next move
            //         edge.x0 += edge.dx;
            //         edge.y0 += edge.dy;
            //     })
            // }
        }
    }
}
