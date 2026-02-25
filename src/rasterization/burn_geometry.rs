/* Rasterize a single geometry */

use crate::{
    encoding::writers::{FillWriter, LineWriter, PixelWriter},
    geo::{
        edges::{LineEdge, PolyEdge, extract_line, extract_point, extract_ring},
        raster::RasterInfo,
    },
    rasterization::{
        burners::{LineBurnStrategy, burn_point, burn_polygon},
        rusterize_impl::PixelCache,
    },
};
use geo_types::{Geometry, GeometryCollection, LineString, MultiLineString, MultiPolygon, Polygon};
use num_traits::Num;

pub trait Burn<T, W>
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T);
}

impl<T, W> Burn<T, W> for Geometry
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T) {
        match self {
            Geometry::Point(geom) => {
                let mut pointedge = Vec::new();
                extract_point(&mut pointedge, geom, raster_info);

                burn_point(pointedge, field_value, writer, background);
            }
            Geometry::MultiPoint(geom) => {
                let mut pointedge = Vec::new();
                for point in geom {
                    extract_point(&mut pointedge, point, raster_info);
                }

                burn_point(pointedge, field_value, writer, background);
            }
            Geometry::Polygon(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::MultiPolygon(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::LineString(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::MultiLineString(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::GeometryCollection(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            _ => (), // not a shapely geometry
        }
    }
}

impl<T, W> Burn<T, W> for GeometryCollection
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T) {
        for geom in self {
            geom.burn::<S>(raster_info, field_value, writer, background)
        }
    }
}

impl<T, W> Burn<T, W> for Polygon
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T) {
        // extract edges
        let mut polyedges = Vec::new();
        extract_ring(&mut polyedges, self.exterior(), raster_info);
        for hole in self.interiors() {
            extract_ring(&mut polyedges, hole, raster_info);
        }

        let (linedges, mut pixel_cache) = if S::IS_ALL_TOUCHED {
            // extract exterior and interior lines
            let mut linedges = Vec::new();
            extract_line(&mut linedges, self.exterior(), raster_info);
            for hole in self.interiors() {
                extract_line(&mut linedges, hole, raster_info);
            }

            let pixel_cache = if S::REQUIRES_DEDUPLICATION {
                Some(PixelCache::new(&linedges))
            } else {
                None
            };

            (Some(linedges), pixel_cache)
        } else {
            (None, None)
        };

        handle_polygon::<T, W, S>(
            raster_info,
            polyedges,
            linedges,
            &mut pixel_cache,
            field_value,
            writer,
            background,
        )
    }
}

impl<T, W> Burn<T, W> for MultiPolygon
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T) {
        // extract edges for all polygon
        let mut polyedges = Vec::new();
        for polygon in self {
            extract_ring(&mut polyedges, polygon.exterior(), raster_info);
            for hole in polygon.interiors() {
                extract_ring(&mut polyedges, hole, raster_info);
            }
        }

        let (linedges, mut pixel_cache) = if S::IS_ALL_TOUCHED {
            // extract exterior and interior lines for all polygons
            let mut linedges = Vec::new();
            for polygon in self {
                extract_line(&mut linedges, polygon.exterior(), raster_info);
                for hole in polygon.interiors() {
                    extract_line(&mut linedges, hole, raster_info);
                }
            }

            let pixel_cache = if S::REQUIRES_DEDUPLICATION {
                Some(PixelCache::new(&linedges))
            } else {
                None
            };

            (Some(linedges), pixel_cache)
        } else {
            (None, None)
        };

        handle_polygon::<T, W, S>(
            raster_info,
            polyedges,
            linedges,
            &mut pixel_cache,
            field_value,
            writer,
            background,
        )
    }
}

impl<T, W> Burn<T, W> for LineString
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T) {
        // extract exterior and interior lines
        let mut linedges = Vec::new();
        extract_line(&mut linedges, self, raster_info);

        // handle cases when pixels are not squares
        if raster_info.xres != raster_info.yres || S::REQUIRES_DEDUPLICATION {
            let mut cache = PixelCache::new(&linedges);
            let mut line_writer = LineWriter::new(writer, &mut cache);
            S::burn_line(linedges, raster_info, field_value, &mut line_writer, background)
        } else {
            S::burn_line(linedges, raster_info, field_value, writer, background)
        }
    }
}

impl<T, W> Burn<T, W> for MultiLineString
where
    T: Num + Copy,
    W: PixelWriter<T>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: T, writer: &mut W, background: T) {
        // extract all edges first to avoid overlaps when a line ends at the beginning of another
        let mut linedges = Vec::new();
        for line in self {
            extract_line(&mut linedges, line, raster_info);
        }

        // handle cases when pixels are not squares
        if raster_info.xres != raster_info.yres || S::REQUIRES_DEDUPLICATION {
            let mut cache = PixelCache::new(&linedges);
            let mut line_writer = LineWriter::new(writer, &mut cache);
            S::burn_line(linedges, raster_info, field_value, &mut line_writer, background)
        } else {
            S::burn_line(linedges, raster_info, field_value, writer, background)
        }
    }
}

fn handle_polygon<T, W, S>(
    raster_info: &RasterInfo,
    polyedges: Vec<PolyEdge>,
    linedges: Option<Vec<LineEdge>>,
    pixel_cache: &mut Option<PixelCache>,
    field_value: T,
    writer: &mut W,
    background: T,
) where
    T: Num + Copy,
    W: PixelWriter<T>,
    S: LineBurnStrategy,
{
    match (linedges, pixel_cache) {
        (Some(lines), Some(cache)) => {
            // pass 1: burn lines
            let mut line_writer = LineWriter::new(writer, cache);
            S::burn_line(lines, raster_info, field_value, &mut line_writer, background);

            // pass 2: fill inner
            let mut fill_writer = FillWriter::new(writer, cache);
            burn_polygon(polyedges, raster_info, field_value, &mut fill_writer, background);
        }
        (Some(lines), None) => {
            S::burn_line(lines, raster_info, field_value, writer, background);
            burn_polygon(polyedges, raster_info, field_value, writer, background);
        }
        (None, _) => {
            burn_polygon(polyedges, raster_info, field_value, writer, background);
        }
    }
}
