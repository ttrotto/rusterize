/*
Build structured edge collection from a single (multi)polygon or (multi)linestring.
If multi or GeometryCollection, then iterates over each inner geometry.
From the Geometry, the values are extracted and reconstructed as an array of nodes.
 */

use crate::geo::{
    edge::{EdgeCollection, LineEdge, PolyEdge},
    raster::RasterInfo,
};

use geo::prelude::*;
use geo_types::{Geometry, LineString};
use numpy::ndarray::Array2;

pub fn build_edges(geom: &Geometry, raster_info: &RasterInfo) -> EdgeCollection {
    let mut edges = EdgeCollection::Empty;
    let mut stack = vec![geom];

    while let Some(current_geom) = stack.pop() {
        match current_geom {
            Geometry::GeometryCollection(collection) => {
                // push geometries back to stack
                for inner in collection {
                    stack.push(inner);
                }
            }
            Geometry::Polygon(polygon) => {
                let mut polyedges: Vec<PolyEdge> = Vec::new();
                process_ring(&mut polyedges, polygon.exterior(), raster_info);
                // process holes in geometry
                for hole in polygon.interiors() {
                    process_ring(&mut polyedges, hole, raster_info);
                }
                edges.add_polyedges(polyedges);
            }
            Geometry::MultiPolygon(multipolygon) => {
                let mut polyedges: Vec<PolyEdge> = Vec::new();
                for polygon in multipolygon {
                    process_ring(&mut polyedges, polygon.exterior(), raster_info);
                    // process holes in geometry
                    for hole in polygon.interiors() {
                        process_ring(&mut polyedges, hole, raster_info);
                    }
                }
                edges.add_polyedges(polyedges);
            }
            Geometry::LineString(line) => {
                let mut linedges: Vec<LineEdge> = Vec::new();
                process_line(&mut linedges, line, raster_info);
                edges.add_linedges(linedges);
            }
            Geometry::MultiLineString(multiline) => {
                let mut linedges: Vec<LineEdge> = Vec::new();
                for line in multiline {
                    process_line(&mut linedges, line, raster_info);
                }
                edges.add_linedges(linedges);
            }
            _ => (),
        }
    }

    edges
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
    let rows = raster_info.nrows as f64;
    let cols = raster_info.ncols as f64;
    for i in 0..nrows {
        // world-to-pixel conversion
        let x0 = (node_array[[i, 0]] - raster_info.xmin) / raster_info.xres;
        let y0 = (raster_info.ymax - node_array[[i, 1]]) / raster_info.yres;

        // TODO: Should this be clamped to to raster size if larger than raster?

        // only add edges that are inside the raster
        if x0 >= 0.0 && x0 < cols && y0 >= 0.0 && y0 < rows {
            edges.push(LineEdge::new(
                node_array[[i, 0]],
                node_array[[i, 1]],
                node_array[[i + 1, 0]],
                node_array[[i + 1, 1]],
                raster_info,
                line.is_closed(),
            ))
        };
    }
}
