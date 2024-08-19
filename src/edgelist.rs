/*
Build edge list from list of polygons or multipolygons. Only PyList objects are supported.
 */

use crate::structs::edge::Edge;
use crate::structs::raster::Raster;

use std::error::Error;
use pyo3::{types::PyList};
use ndarray::Array;

// build list of structured polygon edges from list of polygons
fn build_edges(polylist: &PyList,
               raster: &Raster) -> Result<Vec<Edge>, Box<dyn Error>> {
    // build structured edge for each polygon
    let mut edges = Vec::new();
    for poly in polylist.iter() {
        // init variable for loop
        let (mut y0, mut y1, mut y0c, mut y1c): (f64, f64, f64, f64);

        // extract data from iterator
        let node_vec = poly.extract()?;
        // from list to 2d array of xy pairs
        let n = poly.len()? / 2 - 1;  // drop last entry because duplicate of the first
        let node_array = Array::<f64, _>::from_shape_vec((n, 2), node_vec)
            .expect("Wrong shape vector for node array: {error:?}");
        // add Edge to edges vector
        for i in 0..n {
            y0 = (raster.ymax - node_array[[i, 1]]) / raster.yres - 0.5;
            y1 = (raster.ymax - node_array[[i + 1, 1]]) / raster.yres - 0.5;
            // only add edges that are inside the raster
            if y0 > 0.0 || y1 > 0.0 {
                y0c = y0.ceil();
                y1c = y1.ceil();
                // only add edges if non-horizontal
                if y0c != y1c {
                    edges.push(Edge::new(node_array[[i, 0]], y0, node_array[[i + 1, 0]], y1,
                                         y0c, y1c, raster));
                }
            }
        }
    }
    Ok(edges)
}
