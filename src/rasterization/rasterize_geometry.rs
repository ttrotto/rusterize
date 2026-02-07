/* Rasterize a single (multi)polygon or (multi)linestring */

use crate::{
    encoding::writers::PixelWriter,
    geo::{
        edge::{LineEdge, PolyEdge},
        edge_collection::{EdgeCollection, build_edges},
        raster::RasterInfo,
    },
    prelude::OptFlags,
};
use geo_types::Geometry;
use num_traits::Num;
use rayon::prelude::*;

pub fn rasterize_geometry<T, W>(
    raster_info: &RasterInfo,
    geom: &Geometry,
    field_value: T,
    writer: &mut W,
    background: T,
    opt_flags: &OptFlags,
) where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    // build edge collection
    let edges = build_edges(geom, raster_info, opt_flags);

    match edges {
        // early return if no edges
        EdgeCollection::Empty => (),
        EdgeCollection::PolyEdges(polyedges) => {
            rasterize_polygon(raster_info, polyedges, field_value, writer, background);
        }
        EdgeCollection::LineEdges(linedges) => {
            rasterize_line(linedges, field_value, writer, background, opt_flags);
        }
        EdgeCollection::Mixed {
            polyedges,
            linedges,
        } => {
            rasterize_polygon(raster_info, polyedges, field_value, writer, background);
            rasterize_line(linedges, field_value, writer, background, opt_flags);
        }
    }
}

fn rasterize_polygon<T, W>(
    raster_info: &RasterInfo,
    mut polyedges: Vec<PolyEdge>,
    field_value: T,
    writer: &mut W,
    background: T,
) where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    // sort edges by y coordinate
    polyedges.par_sort_by(|a, b| a.ystart.cmp(&b.ystart));

    // start with first y line
    let mut yline = polyedges.first().unwrap().ystart;

    let mut active_edges: Vec<PolyEdge> = Vec::new();

    // rasterize loop
    let ncols = raster_info.ncols as f64;
    while yline < raster_info.nrows && (!active_edges.is_empty() || !polyedges.is_empty()) {
        // transfer current edges to active edges
        let split_idx = polyedges.partition_point(|edge| edge.ystart <= yline);
        active_edges.extend(polyedges.drain(..split_idx));

        // remove finished edges
        active_edges.retain(|edge| edge.yend > yline);
        if active_edges.is_empty() {
            yline += 1;
            continue;
        }

        // cache x intersection with y line
        for edge in active_edges.iter_mut() {
            edge.x_at_yline = edge.intersect_at(yline);
        }

        // sort by y line
        active_edges.sort_by(|a, b| a.x_at_yline.partial_cmp(&b.x_at_yline).unwrap());

        // fill pixels
        for chunk in active_edges.chunks_exact(2) {
            let x1 = &chunk[0].x_at_yline;
            let x2 = &chunk[1].x_at_yline;

            // round down like GDAL
            let xstart = (x1 + 0.5).floor().clamp(0.0, ncols) as usize;
            let xend = (x2 + 0.5).floor().clamp(0.0, ncols) as usize;

            for xpix in xstart..xend {
                writer.write(yline, xpix, field_value, background);
            }
        }

        yline += 1;
    }
}

fn rasterize_line<T, W>(
    linedges: Vec<LineEdge>,
    field_value: T,
    writer: &mut W,
    background: T,
    opt_flags: &OptFlags,
) where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    let last_idx = linedges.len() - 1;
    for (idx, edge) in linedges.iter().enumerate() {
        let mut x0 = edge.ix0;
        let mut y0 = edge.iy0;

        // rasterize all pixels except very last
        if opt_flags.with_all_touched() {
            let (mut ix, mut iy) = (0, 0);
            let (dx, dy) = (edge.dx, edge.dy.abs());
            while ix < dx || iy < dy {
                writer.write(y0 as usize, x0 as usize, field_value, background);

                let decision = (1 + 2 * ix) * dy - (1 + 2 * iy) * dx;
                if decision == 0 {
                    // exactly diagonal
                    x0 += edge.sx;
                    y0 += edge.sy;
                    ix += 1;
                    iy += 1;
                } else if decision < 0 {
                    // horizonal step
                    x0 += edge.sx;
                    ix += 1;
                } else {
                    // vertical step
                    y0 += edge.sy;
                    iy += 1;
                }
            }
        } else {
            let mut err = edge.dx + edge.dy;
            while x0 != edge.ix1 || y0 != edge.iy1 {
                writer.write(y0 as usize, x0 as usize, field_value, background);

                // update the error term and coordinates
                let e2 = 2 * err;
                if e2 >= edge.dy {
                    err += edge.dy;
                    x0 += edge.sx;
                }
                if e2 <= edge.dx {
                    err += edge.dx;
                    y0 += edge.sy;
                }
            }
        }

        // rasterize last pixel if very last and geometry is not closed
        if idx == last_idx && !edge.is_closed {
            writer.write(y0 as usize, x0 as usize, field_value, background);
        }
    }
}
