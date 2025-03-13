/*
Structure to contain information on polygon and line edges.
 */

use crate::structs::raster::RasterInfo;
use std::cmp::Ordering;

// collection of edges
pub enum EdgeCollection {
    PolyEdges(Vec<PolyEdge>),
    LineEdges(Vec<LineEdge>),
}

impl EdgeCollection {
    pub fn is_empty(&self) -> bool {
        match self {
            EdgeCollection::PolyEdges(poly_edges) => poly_edges.is_empty(),
            EdgeCollection::LineEdges(line_edges) => line_edges.is_empty(),
        }
    }
}

pub struct PolyEdge {
    pub ystart: usize, // first row intersection
    pub yend: usize,   // last row below intersection
    pub x: f64,        // x location of ystart
    pub dxdy: f64,     // step
}

impl PolyEdge {
    pub fn new(
        mut x0: f64,
        y0: f64,
        mut x1: f64,
        y1: f64,
        y0c: f64,
        y1c: f64,
        raster_info: &RasterInfo,
    ) -> Self {
        // get matrix rows and columns from raster info
        x0 = (x0 - raster_info.xmin) / raster_info.xres - 0.5;
        x1 = (x1 - raster_info.xmin) / raster_info.xres - 0.5;

        let (fystart, dxdy, x, yend): (f64, f64, f64, usize);
        // assert edges run from top to bottom of the matrix
        if y1c > y0c {
            fystart = y0c.max(0.0);
            dxdy = (x1 - x0) / (y1 - y0);
            x = x0 + (fystart - y0) * dxdy;
            yend = y1c as usize;
        } else {
            fystart = y1c.max(0.0);
            dxdy = (x0 - x1) / (y0 - y1);
            x = x1 + (fystart - y1) * dxdy;
            yend = y0c as usize;
        }
        let ystart = fystart as usize;
        Self {
            ystart,
            yend,
            x,
            dxdy,
        }
    }
}

// og
// pub struct LineEdge {
//     pub nsteps: usize, // number of steps
//     pub x0: f64,       // metrix column
//     pub y0: f64,       // matrix row
//     pub dx: f64,       // x step
//     pub dy: f64,       // y step
//     pub ystart: f64,   // first y coordinate
// }

// // interpolation
// pub struct LineEdge {
//     pub x0: f64,
//     pub y0: f64,
//     pub x1: f64,
//     pub y1: f64,
//     pub n: usize,
// }

// grid walking
pub struct LineEdge {
    pub ix0: isize,
    pub iy0: isize,
    pub ix1: isize,
    pub iy1: isize,
    pub dx: isize,
    pub dy: isize,
    pub sx: isize,
    pub sy: isize,
    pub err: isize,
    pub is_last_segment: bool,
}

impl LineEdge {
    pub fn new(
        mut x0: f64,
        mut y0: f64,
        mut x1: f64,
        mut y1: f64,
        raster_info: &RasterInfo,
        is_last_segment: bool,
    ) -> Self {
        // world to pixel coordinates
        let (ix0, iy0) = world_to_pixel(x0, y0, &raster_info);
        let (ix1, iy1) = world_to_pixel(x1, y1, &raster_info);
        
        // Calculate differences
        let dx = (ix1 - ix0).abs();
        let dy = -(iy1 - iy0).abs();

        // Determine the direction of the line
        let sx = if ix0 < ix1 { 1 } else { -1 };
        let sy = if iy0 < iy1 { 1 } else { -1 };

        // Initialize the error term
        let mut err = dx + dy;
        
        Self {
            ix0,
            iy0,
            ix1,
            iy1,
            dx,
            dy,
            sx,
            sy,
            err,
            is_last_segment,
        }
        
        // // interpolation
        // let dx = x1 - x0;
        // let dy = y1 - y0;
        // let n = dx.abs().max(dy.abs()).round() as usize;
        // 
        // Self {
        //     x0,
        //     x1,
        //     y0,
        //     y1,
        //     n
        // }
        
        
        // // grid walking
        // let dx = x1 - x0;
        // let dy = y1 - y0;
        // let nx = dx.abs().round() as usize;
        // let ny = dy.abs().round() as usize;
        // let sign_x = if dx > 0.0 { 1.0 } else { -1.0 };
        // let sign_y = if dy > 0.0 { 1.0 } else { -1.0 };
        // 
        // Self {
        //     x0,
        //     y0,
        //     nx,
        //     ny,
        //     sign_x,
        //     sign_y,
        // }
        
        // // get matrix rows and columns from raster info
        // x0 = (x0 - raster_info.xmin) / raster_info.xres - 1.0;
        // x1 = (x1 - raster_info.xmin) / raster_info.xres - 1.0;
        // y0 = (raster_info.ymax - y0) / raster_info.yres - 1.0;
        // y1 = (raster_info.ymax - y1) / raster_info.yres - 1.0;
        //         
        // // calculate steps
        // let mut dx = x1 - x0;
        // let mut dy = y1 - y0;
        // let fnsteps = dx.abs().max(dy.abs()).max(1.0) + 1.0; //.ceil(); // at least 1 step
        // dx /= fnsteps;
        // dy /= fnsteps;
        // let nsteps = fnsteps as usize;
        // let ystart = y0;
        // Self {
        //     nsteps,
        //     x0,
        //     y0,
        //     dx,
        //     dy,
        //     ystart,
        // }
    }
}

fn world_to_pixel(x_world: f64, y_world: f64, raster_info: &RasterInfo) -> (isize, isize) {
    let x_pixel = ((x_world - raster_info.xmin) / raster_info.xres).floor() as isize;
    let y_pixel = ((raster_info.ymax - y_world) / raster_info.yres).floor() as isize;
    (x_pixel, y_pixel)
}
// compare on usize Y coordinate
#[inline]
pub fn less_by_ystart_poly(edge1: &PolyEdge, edge2: &PolyEdge) -> Ordering {
    edge1.ystart.cmp(&edge2.ystart)
}

// partial compare on f64 X coordinate
#[inline]
pub fn less_by_x_poly(edge1: &PolyEdge, edge2: &PolyEdge) -> Ordering {
    edge1.x.partial_cmp(&edge2.x).unwrap_or(Ordering::Equal)
}

// // compare on f64 Y coordinate
// #[inline]
// pub fn less_by_ystart_line(edge1: &LineEdge, edge2: &LineEdge) -> Ordering {
//     edge1
//         .ystart
//         .partial_cmp(&edge2.ystart)
//         .unwrap_or(Ordering::Equal)
// }
// 
// // partial compare on f64 X coordinate
// #[inline]
// pub fn less_by_x_line(edge1: &LineEdge, edge2: &LineEdge) -> Ordering {
//     edge1.x0.partial_cmp(&edge2.x0).unwrap_or(Ordering::Equal)
// }
