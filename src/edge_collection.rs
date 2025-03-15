/*
Build structured edge collection from a single (multi)polygon or (multi)linestring.
If multi, then iterates over each inner geometry.
From the Geometry, the values are extracted and reconstructed as an array of nodes.
 */

use crate::structs::edge::{EdgeCollection, LineEdge, PolyEdge};
use crate::structs::raster::RasterInfo;

use geo::prelude::*;
use geo_types::{Geometry, LineString};
use numpy::ndarray::Array2;

pub fn build_edges(geom: &Geometry, raster_info: &RasterInfo) -> EdgeCollection {
    match geom {
        // polygon
        Geometry::Polygon(polygon) => {
            let mut polyedges: Vec<PolyEdge> = Vec::new();
            // handle exterior polygon
            process_ring(&mut polyedges, polygon.exterior(), raster_info);
            // handle interior polygons (if any)
            for hole in polygon.interiors() {
                process_ring(&mut polyedges, hole, raster_info);
            }
            EdgeCollection::PolyEdges(polyedges)
        }
        // multipolygon - iterate over each inner polygon
        Geometry::MultiPolygon(multipolygon) => {
            let mut polyedges: Vec<PolyEdge> = Vec::new();
            for polygon in multipolygon {
                // handle exterior polygon
                process_ring(&mut polyedges, polygon.exterior(), raster_info);
                // handle interior polygons (if any)
                for hole in polygon.interiors() {
                    process_ring(&mut polyedges, hole, raster_info);
                }
            }
            EdgeCollection::PolyEdges(polyedges)
        }
        // linestring
        Geometry::LineString(line) => {
            let mut linedges: Vec<LineEdge> = Vec::new();
            // handle single segment
            process_line(&mut linedges, line, raster_info);
            EdgeCollection::LineEdges(linedges)
        }
        // multilinestring - iterate over each inner linestring
        Geometry::MultiLineString(multiline) => {
            let mut linedges: Vec<LineEdge> = Vec::new();
            // handle multiple segments
            for line in multiline {
                process_line(&mut linedges, line, raster_info);
            }
            EdgeCollection::LineEdges(linedges)
        }
        _ => unimplemented!("Unsupported geometry type."),
    }
}

fn build_node_array(line: &LineString) -> Array2<f64> {
    // build Nx2 array of nodes (x, y)
    let mut node_array = Array2::<f64>::zeros((line.coords_count(), 2));
    line.coords_iter().enumerate().for_each(|(i, coord)| {
        node_array[[i, 0]] = coord.x;
        node_array[[i, 1]] = coord.y;
    });
    node_array
}

fn process_ring(edges: &mut Vec<PolyEdge>, line: &LineString<f64>, raster_info: &RasterInfo) {
    let node_array = build_node_array(line);
    // drop last entry for correct comperison inside loop
    let nrows = node_array.nrows() - 1;
    // add PolyEdge
    for i in 0..nrows {
        let y0 = (raster_info.ymax - node_array[[i, 1]]) / raster_info.yres - 0.5;
        let y1 = (raster_info.ymax - node_array[[i + 1, 1]]) / raster_info.yres - 0.5;
        // only add edges that are inside the raster
        if y0 > 0.0 || y1 > 0.0 {
            let y0c = y0.ceil();
            let y1c = y1.ceil();
            // only add edges if non-horizontal
            if y0c != y1c {
                edges.push(PolyEdge::new(
                    node_array[[i, 0]],
                    y0,
                    node_array[[i + 1, 0]],
                    y1,
                    y0c,
                    y1c,
                    raster_info,
                ));
            }
        }
    }
}

fn process_line(edges: &mut Vec<LineEdge>, line: &LineString<f64>, raster_info: &RasterInfo) {
    // build node array
    let node_array = build_node_array(line);
    // add LineEdge
    let nrows = node_array.nrows() - 1;
    let irows = raster_info.nrows as isize;
    let icols = raster_info.ncols as isize;
    for i in 0..nrows {
        // world-to-pixel conversion
        let ix0 = ((node_array[[i, 0]] - raster_info.xmin) / raster_info.xres).floor() as isize;
        let iy0 = ((raster_info.ymax - node_array[[i, 1]]) / raster_info.yres).floor() as isize;
        // only add edges that are inside the raster
        if ix0 >= 0 && ix0 < icols && iy0 >= 0 && iy0 < irows {
            edges.push(LineEdge::new(
                ix0,
                iy0,
                node_array[[i + 1, 0]],
                node_array[[i + 1, 1]],
                raster_info,
                line.is_closed(),
            ))
        }
    }
}
