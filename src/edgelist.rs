/*
Build structured edge list from list of polygons.
Input looks like this:
[ [ ... ]
  [ ... ]
  [ ... ] ]
Where each sublist is [x1 y1 x2 y2 ... xn yn], which is first
converted into a Nx2 matrix of nodes and then an edge list is
built on top of that
 */

use crate::structs::edge::Edge;
use crate::structs::raster::Raster;

use std::error::Error;
use ndarray::Array;

pub fn build_edges(vpoly: Vec<Vec<f64>>,
                   raster: &Raster) -> Result<Vec<Edge>, Box<dyn Error>> {
    // build structured edge for each polygon
    let mut edges = Vec::new();
    for poly in vpoly.into_iter() {
        // from vector to 2d array of xy pairs
        let nrows = (poly.len() - 2) / 2;  // drop last 2 entries because duplicates
        let node_array = Array::<f64, _>::from_shape_vec((nrows, 2), poly)
            .expect("Wrong shape vector for node array: {error:?}");
        // add Edge to edges vector
        for i in 0..nrows {
            let y0 = (raster.ymax - node_array[[i, 1]]) / raster.yres - 0.5;
            let y1 = (raster.ymax - node_array[[i + 1, 1]]) / raster.yres - 0.5;
            // only add edges that are inside the raster
            if y0 > 0.0 || y1 > 0.0 {
                let y0c = y0.ceil();
                let y1c = y1.ceil();
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