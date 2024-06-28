/*
Structures to contain information on raster data and edges.
*/

// Raster information
struct Raster {
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    nrow: f64,
    ncol: f64,
    xres: f64,
    yres: f64
}

impl Raster {
    // initialize
    fn init(xmin: f64,
            xmax: f64,
            ymin: f64,
            ymax: f64,
            nrow: f64,
            ncol: f64) -> Self {
        // raster resolution
        let xres = (xmax - xmin) / ncol;
        let yres = (ymax - ymin) / ncol;
        Self {
            xmin,
            xmax,
            ymin,
            ymax,
            nrow,
            ncol,
            xres,
            yres,
        }
    }
}

// Polygon information
struct Edge {
    ystart: f64,  // matrix first intersection
    yend: f64,  // matrix last intersection
    xstart: f64,  // x location of ystart
    dxdy: f64,  // step
}

impl Edge {
    // initialize
    fn init(mut x0: f64,
            y0: f64,
            mut x1: f64,
            y1: f64,
            raster: &Raster,
            y0c: f64,
            y1c: f64) -> Self {
        // get matrix rows and columns from resolution
        x0 = (x0 - raster.xmin) / raster.xres - 0.5;
        x1 = (x1 - raster.xmin) / raster.xres - 0.5;
        // init structure keys
        let (ystart, dxdy, xstart, yend);
        // assert edges run from top to bottom of the matrix
        if y1c > y0c {
            ystart = f64::max(y0c,0.0);
            dxdy = (x1 - x0) / (y1 - y0);
            xstart = x0 + (ystart - y0) * dxdy;
            yend = y1c;
        } else {
            ystart = f64::max(y1c,0.0);
            dxdy = (x0 - x1) / (y0 - y1);
            xstart = x1 + (ystart - y1) * dxdy;
            yend = y0c;
        }
        Self {
            ystart,
            yend,
            xstart,
            dxdy
        }
    }
    // compare on Y coordinate
    fn cmp_y(edge1: &Edge,
             edge2: &Edge) -> bool {
        edge1.ystart < edge2.ystart
    }
    // compare on X coordinate
    fn cmp_x(edge1: &Edge,
             edge2: &Edge) -> bool {
        edge1.ystart < edge2.ystart
    }
}