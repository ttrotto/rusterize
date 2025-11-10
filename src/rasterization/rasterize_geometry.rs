/* Rasterize a single (multi)polygon or (multi)linestring */

use crate::{
    encoding::writers::PixelWriter,
    geo::{
        edge::{EdgeCollection, LineEdge, PolyEdge, less_by_x, less_by_ystart},
        edge_collection,
        raster::RasterInfo,
    },
};

use edge_collection::build_edges;
use geo_types::Geometry;
use num_traits::Num;
use rayon::prelude::*;

pub fn rasterize_geometry<T, W>(
    raster_info: &RasterInfo,
    geom: &Geometry,
    field_value: T,
    writer: &mut W,
    background: T,
) where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    // build edge collection
    let edges = build_edges(geom, raster_info);

    match edges {
        // early return if no edges
        EdgeCollection::Empty => (),
        EdgeCollection::PolyEdges(polyedges) => {
            rasterize_polygon(raster_info, polyedges, field_value, writer, background);
        }
        EdgeCollection::LineEdges(linedges) => {
            rasterize_line(linedges, field_value, writer, background);
        }
        EdgeCollection::Mixed {
            polyedges,
            linedges,
        } => {
            rasterize_polygon(raster_info, polyedges, field_value, writer, background);
            rasterize_line(linedges, field_value, writer, background);
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
    // sort edges
    polyedges.par_sort_by(less_by_ystart);

    // start with first y line
    let mut yline = polyedges.first().unwrap().ystart;

    // active edges
    let mut active_edges = Vec::new();

    // rasterize loop
    let ncols = raster_info.ncols as f64;
    while yline < raster_info.nrows && !(active_edges.is_empty() && polyedges.is_empty()) {
        // transfer current edges to active edges
        active_edges.extend(polyedges.extract_if(.., |edge| edge.ystart <= yline));
        // sort active edges
        active_edges.par_sort_by(less_by_x);

        // even-odd polygon fill
        for (edge1, edge2) in active_edges
            .iter()
            .zip(active_edges.iter().skip(1))
            .step_by(2)
        {
            // clamp the x-coordinates of the edges
            let xstart = edge1.x.clamp(0.0, ncols).ceil() as usize;
            let xend = edge2.x.clamp(0.0, ncols).ceil() as usize;

            // fill the pixels between xstart and xend
            for xpix in xstart..xend {
                writer.write(yline, xpix, field_value, background);
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

fn rasterize_line<T, W>(mut linedges: Vec<LineEdge>, field_value: T, writer: &mut W, background: T)
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    let last_idx = linedges.len() - 1;
    for (idx, edge) in linedges.iter_mut().enumerate() {
        // rasterize all pixels except very last
        while edge.ix0 != edge.ix1 || edge.iy0 != edge.iy1 {
            let ix0 = edge.ix0 as usize;
            let iy0 = edge.iy0 as usize;
            writer.write(iy0, ix0, field_value, background);

            // update the error term and coordinates
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

        // rasterize last pixel if very last and geometry is not closed
        if idx == last_idx && !edge.is_closed {
            let ix0 = edge.ix0 as usize;
            let iy0 = edge.iy0 as usize;
            writer.write(iy0, ix0, field_value, background);
        }
    }
}
