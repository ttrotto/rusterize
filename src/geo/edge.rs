/* Structure to contain information on polygon and line edges */

use crate::geo::raster::RasterInfo;
use std::cmp::Ordering;

// collection of edges
pub enum EdgeCollection {
    Empty,
    PolyEdges(Vec<PolyEdge>),
    LineEdges(Vec<LineEdge>),
    Mixed {
        polyedges: Vec<PolyEdge>,
        linedges: Vec<LineEdge>,
    },
}

impl EdgeCollection {
    pub fn add_polyedges(&mut self, new_polyedges: Vec<PolyEdge>) {
        if new_polyedges.is_empty() {
            return;
        }
        match self {
            EdgeCollection::Empty => *self = EdgeCollection::PolyEdges(new_polyedges),
            EdgeCollection::PolyEdges(polyedges) => polyedges.extend(new_polyedges),
            EdgeCollection::LineEdges(linedges) => {
                *self = {
                    EdgeCollection::Mixed {
                        polyedges: new_polyedges,
                        linedges: std::mem::take(linedges),
                    }
                }
            }
            EdgeCollection::Mixed { polyedges, .. } => polyedges.extend(new_polyedges),
        }
    }

    pub fn add_linedges(&mut self, new_linedges: Vec<LineEdge>) {
        if new_linedges.is_empty() {
            return;
        }
        match self {
            EdgeCollection::Empty => *self = EdgeCollection::LineEdges(new_linedges),
            EdgeCollection::PolyEdges(polyedges) => {
                *self = {
                    EdgeCollection::Mixed {
                        polyedges: std::mem::take(polyedges),
                        linedges: new_linedges,
                    }
                }
            }
            EdgeCollection::LineEdges(linedges) => linedges.extend(new_linedges),
            EdgeCollection::Mixed { linedges, .. } => linedges.extend(new_linedges),
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
        // world-to-pixel conversion
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

pub struct LineEdge {
    pub ix0: isize,
    pub iy0: isize,
    pub ix1: isize,
    pub iy1: isize,
    pub dx: isize, // horizontal step
    pub dy: isize, // vertical step
    pub sx: isize, // horizontal slope
    pub sy: isize, // vertical slope
    pub is_closed: bool,
}

impl LineEdge {
    pub fn new(
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        raster_info: &RasterInfo,
        is_closed: bool,
    ) -> Self {
        // world-to-pixel conversion
        let ix0 = ((x0 - raster_info.xmin) / raster_info.xres).floor() as isize;
        let iy0 = ((raster_info.ymax - y0) / raster_info.yres).floor() as isize;
        let ix1 = ((x1 - raster_info.xmin) / raster_info.xres).floor() as isize;
        let iy1 = ((raster_info.ymax - y1) / raster_info.yres).floor() as isize;

        // calculate steps
        let dx = (ix1 - ix0).abs();
        let dy = -(iy1 - iy0).abs();

        // determine the direction of the line
        let sx = if ix0 < ix1 { 1 } else { -1 };
        let sy = if iy0 < iy1 { 1 } else { -1 };

        Self {
            ix0,
            iy0,
            ix1,
            iy1,
            dx,
            dy,
            sx,
            sy,
            is_closed,
        }
    }
}

// compare on usize Y coordinate for polygons
#[inline]
pub fn less_by_ystart(edge1: &PolyEdge, edge2: &PolyEdge) -> Ordering {
    edge1.ystart.cmp(&edge2.ystart)
}

// partial compare on f64 X coordinate for polygons
#[inline]
pub fn less_by_x(edge1: &PolyEdge, edge2: &PolyEdge) -> Ordering {
    edge1.x.partial_cmp(&edge2.x).unwrap_or(Ordering::Equal)
}
