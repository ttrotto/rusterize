/*
The AllTouched strategy has been adapted from GDAL: https://github.com/OSGeo/gdal/blob/63396dbf42999441478e036ebb145725de09f7ce/alg/llrasterize.cpp#L407
Primarily for output consistency.
*/

use crate::{
    encoding::writers::PixelWriter,
    geo::{
        edges::{LineEdge, PointEdge, PolyEdge},
        raster::RasterInfo,
    },
};
use num_traits::Num;
use rayon::prelude::*;

pub struct Standard;
pub struct AllTouched;

const EPSILON_INTERSECT: f64 = 1e-4;
const TOLERANCE: f64 = 1e-9;

pub trait LineBurnStrategy {
    const IS_ALL_TOUCHED: bool;

    fn burn_line<T, W>(
        linedges: Vec<LineEdge>,
        raster_info: &RasterInfo,
        field_value: T,
        writer: &mut W,
        background: T,
    ) where
        T: Num + Copy,
        W: PixelWriter<T>;
}

impl LineBurnStrategy for Standard {
    const IS_ALL_TOUCHED: bool = false;

    fn burn_line<T, W>(linedges: Vec<LineEdge>, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T)
    where
        T: Num + Copy,
        W: PixelWriter<T>,
    {
        // early return if empty
        if linedges.is_empty() {
            return;
        }

        let nrows = raster_info.nrows as isize;
        let ncols = raster_info.ncols as isize;
        let last_idx = linedges.len() - 1;

        for (idx, edge) in linedges.iter().enumerate() {
            let mut ix0 = edge.x0.floor() as isize;
            let ix1 = edge.x1.floor() as isize;
            let mut iy0 = edge.y0.floor() as isize;
            let iy1 = edge.y1.floor() as isize;

            // steps
            let dx = (ix1 - ix0).abs();
            let dy = -(iy1 - iy0).abs();

            // direction of the line
            let sx = if ix0 < ix1 { 1 } else { -1 };
            let sy = if iy0 < iy1 { 1 } else { -1 };

            // write
            let mut err = dx + dy;
            while ix0 != ix1 || iy0 != iy1 {
                if ix0 >= 0 && ix0 < ncols && iy0 >= 0 && iy0 < nrows {
                    writer.write(iy0 as usize, ix0 as usize, field_value, background);
                }

                // update the error term and coordinates
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    ix0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    iy0 += sy;
                }
            }

            // rasterize last pixel if very last and geometry is not closed
            if idx == last_idx && !edge.is_closed && ix0 >= 0 && ix0 < ncols && iy0 >= 0 && iy0 < nrows {
                writer.write(iy0 as usize, ix0 as usize, field_value, background);
            }
        }
    }
}

impl LineBurnStrategy for AllTouched {
    const IS_ALL_TOUCHED: bool = true;

    fn burn_line<T, W>(linedges: Vec<LineEdge>, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T)
    where
        T: Num + Copy,
        W: PixelWriter<T>,
    {
        // early return if empty
        if linedges.is_empty() {
            return;
        }

        let nrows = raster_info.nrows as isize;
        let ncols = raster_info.ncols as isize;
        let nrows_f64 = raster_info.nrows as f64;
        let ncols_f64 = raster_info.ncols as f64;

        for edge in linedges.iter() {
            let mut df_x = edge.x0;
            let mut df_y = edge.y0;
            let mut df_x_end = edge.x1;
            let mut df_y_end = edge.y1;

            // proceed left-to-right
            if df_x > df_x_end {
                std::mem::swap(&mut df_x, &mut df_x_end);
                std::mem::swap(&mut df_y, &mut df_y_end);
            }

            // vertical lines
            if (df_x - df_x_end).abs() < 0.01 {
                if df_y_end < df_y {
                    std::mem::swap(&mut df_y, &mut df_y_end);
                }

                let ix = df_x_end.floor() as isize;
                let mut iy = df_y.floor() as isize;
                let mut iy_end = (df_y_end - EPSILON_INTERSECT).floor() as isize;

                if ix < 0 || ix >= ncols {
                    continue;
                }

                // clamp to raster size
                iy = iy.max(0);
                iy_end = iy_end.min(nrows - 1);

                // write
                for y in iy..=iy_end {
                    writer.write(y as usize, ix as usize, field_value, background);
                }

                // next segment
                continue;
            }

            // horizontal lines
            if (df_y - df_y_end).abs() < 0.01 {
                if df_x_end < df_x {
                    std::mem::swap(&mut df_x, &mut df_x_end);
                }

                let mut ix = df_x.floor() as isize;
                let iy = df_y.floor() as isize;
                let mut ix_end = (df_x_end - EPSILON_INTERSECT).floor() as isize;

                if iy < 0 || iy >= nrows {
                    continue;
                }

                // clamp to raster size
                ix = ix.max(0);
                ix_end = ix_end.min(ncols - 1);

                // writer
                for x in ix..=ix_end {
                    writer.write(iy as usize, x as usize, field_value, background);
                }

                // next segment
                continue;
            }

            // sloped line
            let slope = (df_y_end - df_y) / (df_x_end - df_x);
            let inv_slope = 1.0 / slope;

            // clip along x axis
            if df_x < 0.0 {
                df_y += (0.0 - df_x) * slope;
                df_x = 0.0;
            }
            if df_x_end > ncols_f64 {
                df_y_end += (ncols_f64 - df_x_end) * slope;
                df_x_end = ncols_f64;
            }

            // clip along y axis
            if df_y < 0.0 {
                df_x += (0.0 - df_y) * inv_slope;
                df_y = 0.0;
            } else if df_y > nrows_f64 {
                df_x += (nrows_f64 - df_y) * inv_slope;
                df_y = nrows_f64;
            }

            if df_y_end < 0.0 {
                df_x_end += (0.0 - df_y_end) * inv_slope;
            } else if df_y_end > nrows_f64 {
                df_x_end += (nrows_f64 - df_y_end) * inv_slope;
            }

            // clamp to raster size
            df_x = df_x.clamp(0.0, ncols_f64);
            df_x_end = df_x_end.clamp(0.0, ncols_f64);

            // write
            while df_x >= 0.0 && df_x < df_x_end {
                let ix = df_x.floor() as isize;
                let iy = df_y.floor() as isize;

                if ix >= 0 && ix < ncols && iy >= 0 && iy < nrows {
                    writer.write(iy as usize, ix as usize, field_value, background);
                }

                let mut sx = (df_x + 1.0).floor() - df_x;
                let mut sy = sx * slope;

                if (df_y + sy).floor() as isize == iy {
                    df_x += sx;
                    df_y += sy;
                } else if slope < 0.0 {
                    sy = iy as f64 - df_y;
                    if sy > -TOLERANCE {
                        sy = -TOLERANCE;
                    }
                    sx = sy / slope;
                    df_x += sx;
                    df_y += sy;
                } else {
                    sy = (iy + 1) as f64 - df_y;
                    if sy < TOLERANCE {
                        sy = TOLERANCE;
                    }
                    sx = sy / slope;
                    df_x += sx;
                    df_y += sy;
                }
            }
        }
    }
}

pub fn burn_point<T, W>(pointedges: Vec<PointEdge>, field_value: T, writer: &mut W, background: T)
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    for point in pointedges {
        writer.write(point.y, point.x, field_value, background);
    }
}

pub fn burn_polygon<T, W>(
    mut polyedges: Vec<PolyEdge>,
    raster_info: &RasterInfo,
    field_value: T,
    writer: &mut W,
    background: T,
) where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    // early return if empty
    if polyedges.is_empty() {
        return;
    }

    // sort edges by y coordinate
    polyedges.par_sort_by(|a, b| a.ystart.cmp(&b.ystart));

    // start with first y line
    let mut yline = polyedges.first().unwrap().ystart;

    let mut active_edges: Vec<PolyEdge> = Vec::new();

    // rasterize loop
    let ncols = raster_info.ncols as f64;
    while yline < raster_info.nrows && (!active_edges.is_empty() || !polyedges.is_empty()) {
        // transfer current edges to active edges
        let split_idx = polyedges.partition_point(|edge| edge.ystart <= yline);
        active_edges.extend(polyedges.drain(..split_idx));

        // remove finished edges
        active_edges.retain(|edge| edge.yend > yline);
        if active_edges.is_empty() {
            yline += 1;
            continue;
        }

        // cache x intersection with y line
        for edge in active_edges.iter_mut() {
            edge.x_at_yline = edge.intersect_at(yline);
        }

        // sort by y line
        active_edges.par_sort_by(|a, b| a.x_at_yline.partial_cmp(&b.x_at_yline).unwrap());

        // fill pixels
        for chunk in active_edges.chunks_exact(2) {
            let x1 = &chunk[0].x_at_yline;
            let x2 = &chunk[1].x_at_yline;

            // round down like GDAL
            let xstart = (x1 + 0.5).floor().clamp(0.0, ncols) as usize;
            let xend = (x2 + 0.5).floor().clamp(0.0, ncols) as usize;

            for xpix in xstart..xend {
                writer.write(yline, xpix, field_value, background);
            }
        }

        yline += 1;
    }
}
