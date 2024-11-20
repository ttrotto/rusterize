/*
Structure to contain information on polygon edges.
 */

use std::cmp::Ordering;
use crate::structs::raster::Raster;

#[derive(Clone)]
pub struct Edge {
    pub ystart: usize,  // first row intersection
    pub yend: usize,  // last row below intersection
    pub x: f64,  // x location of ystart
    pub dxdy: f64,  // step
}

impl Edge {
    pub fn new(mut x0: f64,
               y0: f64,
               mut x1: f64,
               y1: f64,
               y0c: f64,
               y1c: f64,
               raster: &Raster) -> Self {
        // get matrix rows and columns from resolution
        x0 = (x0 - raster.xmin) / raster.xres - 0.5;
        x1 = (x1 - raster.xmin) / raster.xres - 0.5;
        // init structure keys
        let (fystart, dxdy, x, yend): (f64, f64, f64, usize);
        // assert edges run from top to bottom of the matrix
        if y1c > y0c {
            fystart = f64::max(y0c, 0.0);
            dxdy = (x1 - x0) / (y1 - y0);
            x = x0 + (fystart - y0) * dxdy;
            yend = y1c as usize;
        } else {
            fystart = f64::max(y1c, 0.0);
            dxdy = (x0 - x1) / (y0 - y1);
            x = x1 + (fystart - y1) * dxdy;
            yend = y0c as usize;
        }
        // safe type casting
        let ystart = fystart as usize;
        Self {
            ystart,
            yend,
            x,
            dxdy,
        }
    }
}

// compare on usize Y coordinate
pub fn less_by_ystart(edge1: &Edge,
                      edge2: &Edge) -> Ordering {
    edge1.ystart.cmp(&edge2.ystart)
}
// partial compare on f64 X coordinate
pub fn less_by_x(edge1: &Edge,
                 edge2: &Edge) -> Ordering {
    edge1.x.partial_cmp(&edge2.x).unwrap_or(Ordering::Equal)  // treat NaN as equal for sorting
}