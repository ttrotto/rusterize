/*
Build structured edge list from a single (multi)polygon.
If multipolygon, then iterates over each inner polygon.
From the Geometry, the values are extracted and reconstructed as an array of nodes.
 */

use crate::structs::edge::Edge;
use crate::structs::raster::RasterInfo;

use geo::prelude::*;
use geo_types::{Geometry, LineString};
use numpy::ndarray::Array2;

pub fn build_edges(edges: &mut Vec<Edge>, polygon: &Geometry, raster_info: &RasterInfo) {
    match polygon {
        // polygon
        Geometry::Polygon(polygon) => {
            // handle exterior polygon
            process_ring(edges, polygon.exterior(), raster_info);
            // handle interior polygons (if any)
            for hole in polygon.interiors() {
                process_ring(edges, hole, raster_info);
            }
        }
        // multipolygon - iterate over each inner polygon
        Geometry::MultiPolygon(polygon) => {
            for poly in polygon {
                build_edges(edges, &Geometry::Polygon(poly.clone()), raster_info);
            }
        }
        _ => unimplemented!("Only Polygon and MultiPolygon geometries are supported."),
    }
}

fn process_ring(edges: &mut Vec<Edge>, ring: &LineString<f64>, raster_info: &RasterInfo) {
    // build Nx2 array of nodes (x, y)
    let mut node_array = Array2::<f64>::zeros((ring.coords_count(), 2));
    ring.coords_iter().enumerate().for_each(|(i, coord)| {
        node_array[[i, 0]] = coord.x;
        node_array[[i, 1]] = coord.y;
    });
    // drop last entry for correct comperison inside loop
    let nrows = node_array.nrows() - 1;
    // add Edge to edges vector
    for i in 0..nrows {
        let y0 = (raster_info.ymax - node_array[[i, 1]]) / raster_info.yres - 0.5;
        let y1 = (raster_info.ymax - node_array[[i + 1, 1]]) / raster_info.yres - 0.5;
        // only add edges that are inside the raster
        if y0 > 0.0 || y1 > 0.0 {
            let y0c = y0.ceil();
            let y1c = y1.ceil();
            // only add edges if non-horizontal
            if y0c != y1c {
                edges.push(Edge::new(
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
