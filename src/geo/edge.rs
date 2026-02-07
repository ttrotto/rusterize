/* Structure to contain information on polygon and line edges */

use crate::geo::raster::RasterInfo;

pub struct PolyEdge {
    pub ystart: usize,
    pub yend: usize,
    x0: f64,
    y0: f64,
    dxdy: f64,           // slope
    pub x_at_yline: f64, // x intersection with y line,
}

impl PolyEdge {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        // make sure we go from top to bottom
        let (x_top, y_top, x_bot, y_bot) = if y0 < y1 {
            (x0, y0, x1, y1)
        } else {
            (x1, y1, x0, y0)
        };

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
            x_at_yline: f64::NAN, // dummy
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
