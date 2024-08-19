/*
Structure to contain information on polygon edges.
 */

use crate::structs::raster::Raster;

pub struct Edge {
    ystart: usize,  // first row intersection
    yend: usize,  // last row below intersection
    xstart: f64,  // x location of ystart
    dxdy: f64,  // step
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
        let (fystart, dxdy, xstart, yend): (f64, f64, f64, usize);
        // assert edges run from top to bottom of the matrix
        if y1c > y0c {
            fystart = f64::max(y0c, 0.0);
            dxdy = (x1 - x0) / (y1 - y0);
            xstart = x0 + (fystart - y0) * dxdy;
            yend = y1c as usize;
        } else {
            fystart = f64::max(y1c, 0.0);
            dxdy = (x0 - x1) / (y0 - y1);
            xstart = x1 + (fystart - y1) * dxdy;
            yend = y0c as usize;
        }
        // safe type casting
        let ystart = fystart as usize;
        Self {
            ystart,
            yend,
            xstart,
            dxdy,
        }
    }
}

// compare on Y coordinate
pub fn less_by_ystart(edge1: &Edge,
                      edge2: &Edge) -> bool {
    edge1.ystart < edge2.ystart
}
// compare on X coordinate
pub fn less_by_x(edge1: &Edge,
                 edge2: &Edge) -> bool {
    edge1.ystart < edge2.ystart
}