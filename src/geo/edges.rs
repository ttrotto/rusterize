/* Structure to contain information on geometry edges */

use crate::geo::raster::RasterInfo;
use geo_types::{LineString, Point};

pub struct PointEdge {
    pub x: usize,
    pub y: usize,
}

impl PointEdge {
    fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

pub struct PolyEdge {
    pub ystart: usize,
    pub yend: usize,
    x0: f64,
    y0: f64,
    dxdy: f64,           // slope
    pub x_at_yline: f64, // x intersection with y line,
}

impl PolyEdge {
    fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        // make sure we go from top to bottom
        let (x_top, y_top, x_bot, y_bot) = if y0 < y1 { (x0, y0, x1, y1) } else { (x1, y1, x0, y0) };

        // first and last y lines
        let ystart = (y_top - 0.5).ceil() as usize;
        let yend = (y_bot - 0.5).ceil() as usize;

        // slope
        let dxdy = (x_bot - x_top) / (y_bot - y_top);

        Self {
            ystart,
            yend,
            x0: x_top,
            y0: y_top,
            dxdy,
            x_at_yline: f64::INFINITY, // dummy
        }
    }

    // sort by x intersection at y line
    #[inline]
    pub fn intersect_at(&self, yline: usize) -> f64 {
        // y line center
        let center_y = yline as f64 + 0.5;

        self.x0 + (center_y - self.y0) * self.dxdy
    }
}

pub struct LineEdge {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub is_closed: bool,
}

impl LineEdge {
    fn new(x0: f64, y0: f64, x1: f64, y1: f64, is_closed: bool) -> Self {
        Self {
            x0,
            y0,
            x1,
            y1,
            is_closed,
        }
    }
}

pub fn extract_point(edges: &mut Vec<PointEdge>, point: &Point<f64>, raster_info: &RasterInfo) {
    // world-to-pixel conversion
    let x = (point.x() - raster_info.xmin) / raster_info.xres;
    let y = (raster_info.ymax - point.y()) / raster_info.yres;

    // only keep if inside raster
    if x >= 0.0 && x < raster_info.ncols as f64 && y >= 0.0 && y < raster_info.nrows as f64 {
        edges.push(PointEdge::new(x as usize, y as usize));
    }
}

pub fn extract_ring(edges: &mut Vec<PolyEdge>, line: &LineString<f64>, raster_info: &RasterInfo) {
    let rows = raster_info.nrows as f64;
    for w in line.0.windows(2) {
        // world-to-pixel conversion
        let x0 = (w[0].x - raster_info.xmin) / raster_info.xres;
        let y0 = (raster_info.ymax - w[0].y) / raster_info.yres;
        let x1 = (w[1].x - raster_info.xmin) / raster_info.xres;
        let y1 = (raster_info.ymax - w[1].y) / raster_info.yres;

        // skip horizontal
        if (y0 - y1).abs() >= f64::EPSILON {
            let min_y = y0.min(y1);
            let max_y = y0.max(y1);

            // only keep if inside the raster
            if min_y < rows && max_y >= 0.0 {
                edges.push(PolyEdge::new(x0, y0, x1, y1));
            }
        }
    }
}

pub fn extract_line(edges: &mut Vec<LineEdge>, line: &LineString<f64>, raster_info: &RasterInfo) {
    let rows = raster_info.nrows as f64;
    let cols = raster_info.ncols as f64;
    let is_closed = line.is_closed();

    for w in line.0.windows(2) {
        // world-to-pixel conversion
        let x0 = (w[0].x - raster_info.xmin) / raster_info.xres;
        let y0 = (raster_info.ymax - w[0].y) / raster_info.yres;
        let x1 = (w[1].x - raster_info.xmin) / raster_info.xres;
        let y1 = (raster_info.ymax - w[1].y) / raster_info.yres;

        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);

        // only keep if inside the raster
        if min_x < cols && max_x >= 0.0 && min_y < rows && max_y >= 0.0 {
            edges.push(LineEdge::new(x0, y0, x1, y1, is_closed));
        }
    }
}
