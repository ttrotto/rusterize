use crate::{
    encoding::writers::{FillWriter, LineWriter, PixelWriter},
    geo::{
        edges::{LineEdge, PolyEdge, extract_line, extract_point, extract_ring},
        raster::RasterInfo,
    },
    rasterization::{
        burners::{LineBurnStrategy, burn_point, burn_polygon},
        pixel_cache::PixelCache,
    },
};
use geo_types::{Geometry, GeometryCollection, LineString, MultiLineString, MultiPolygon, Polygon};
use num_traits::Num;

/// Burn a single [`geo::Geometry`] onto a [`DenseArray`] or [`SparseArray`].
pub(crate) trait Burn<N, W>
where
    N: Num + Copy,
    W: PixelWriter<N>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: N, writer: &mut W, background: N);
}

impl<N, W> Burn<N, W> for Geometry
where
    N: Num + Copy,
    W: PixelWriter<N>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: N, writer: &mut W, background: N) {
        match self {
            Geometry::Point(geom) => {
                let mut pointedge = Vec::new();
                extract_point(&mut pointedge, geom, raster_info);

                burn_point(&pointedge, field_value, writer, background);
            }
            Geometry::MultiPoint(geom) => {
                let mut pointedge = Vec::new();
                for point in geom {
                    extract_point(&mut pointedge, point, raster_info);
                }

                burn_point(&pointedge, field_value, writer, background);
            }
            Geometry::Polygon(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::MultiPolygon(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::LineString(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::MultiLineString(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::GeometryCollection(geom) => geom.burn::<S>(raster_info, field_value, writer, background),
            Geometry::Rect(geom) => geom
                .to_polygon()
                .burn::<S>(raster_info, field_value, writer, background),
            Geometry::Triangle(geom) => geom
                .to_polygon()
                .burn::<S>(raster_info, field_value, writer, background),
            Geometry::Line(geom) => {
                let linestring = LineString::new(vec![geom.start, geom.end]);
                linestring.burn::<S>(raster_info, field_value, writer, background)
            }
        }
    }
}

impl<N, W> Burn<N, W> for GeometryCollection
where
    N: Num + Copy,
    W: PixelWriter<N>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: N, writer: &mut W, background: N) {
        for geom in self {
            geom.burn::<S>(raster_info, field_value, writer, background)
        }
    }
}

impl<N, W> Burn<N, W> for Polygon
where
    N: Num + Copy,
    W: PixelWriter<N>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: N, writer: &mut W, background: N) {
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

            let pixel_cache = if S::REQUIRES_DEDUP {
                Some(PixelCache::new(&linedges))
            } else {
                None
            };

            (Some(linedges), pixel_cache)
        } else {
            (None, None)
        };

        handle_polygon::<N, W, S>(
            raster_info,
            &mut polyedges,
            linedges,
            &mut pixel_cache,
            field_value,
            writer,
            background,
        )
    }
}

impl<N, W> Burn<N, W> for MultiPolygon
where
    N: Num + Copy,
    W: PixelWriter<N>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: N, writer: &mut W, background: N) {
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

            let pixel_cache = if S::REQUIRES_DEDUP {
                Some(PixelCache::new(&linedges))
            } else {
                None
            };

            (Some(linedges), pixel_cache)
        } else {
            (None, None)
        };

        handle_polygon::<N, W, S>(
            raster_info,
            &mut polyedges,
            linedges,
            &mut pixel_cache,
            field_value,
            writer,
            background,
        )
    }
}

impl<N, W> Burn<N, W> for LineString
where
    N: Num + Copy,
    W: PixelWriter<N>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: N, writer: &mut W, background: N) {
        // extract exterior and interior lines
        let mut linedges = Vec::new();
        extract_line(&mut linedges, self, raster_info);

        // handle cases when pixels are not squares
        if raster_info.xres != raster_info.yres || S::REQUIRES_DEDUP {
            let mut cache = PixelCache::new(&linedges);
            let mut line_writer = LineWriter::new(writer, &mut cache);
            S::burn_line(&linedges, raster_info, field_value, &mut line_writer, background)
        } else {
            S::burn_line(&linedges, raster_info, field_value, writer, background)
        }
    }
}

impl<N, W> Burn<N, W> for MultiLineString
where
    N: Num + Copy,
    W: PixelWriter<N>,
{
    fn burn<S: LineBurnStrategy>(&self, raster_info: &RasterInfo, field_value: N, writer: &mut W, background: N) {
        // extract all edges first to avoid overlaps when a line ends at the beginning of another
        let mut linedges = Vec::new();
        for line in self {
            extract_line(&mut linedges, line, raster_info);
        }

        // handle cases when pixels are not squares
        if raster_info.xres != raster_info.yres || S::REQUIRES_DEDUP {
            let mut cache = PixelCache::new(&linedges);
            let mut line_writer = LineWriter::new(writer, &mut cache);
            S::burn_line(&linedges, raster_info, field_value, &mut line_writer, background)
        } else {
            S::burn_line(&linedges, raster_info, field_value, writer, background)
        }
    }
}

fn handle_polygon<N, W, S>(
    raster_info: &RasterInfo,
    polyedges: &mut Vec<PolyEdge>,
    linedges: Option<Vec<LineEdge>>,
    pixel_cache: &mut Option<PixelCache>,
    field_value: N,
    writer: &mut W,
    background: N,
) where
    N: Num + Copy,
    W: PixelWriter<N>,
    S: LineBurnStrategy,
{
    match (linedges, pixel_cache) {
        (Some(lines), Some(cache)) => {
            // pass 1: burn lines
            let mut line_writer = LineWriter::new(writer, cache);
            S::burn_line(&lines, raster_info, field_value, &mut line_writer, background);

            // pass 2: fill inner
            let mut fill_writer = FillWriter::new(writer, cache);
            burn_polygon(polyedges, raster_info, field_value, &mut fill_writer, background);
        }
        (Some(lines), None) => {
            S::burn_line(&lines, raster_info, field_value, writer, background);
            burn_polygon(polyedges, raster_info, field_value, writer, background);
        }
        (None, _) => {
            burn_polygon(polyedges, raster_info, field_value, writer, background);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encoding::writers::PixelWriter, rasterization::burners::Standard};
    use geo_types::{Line, MultiPoint, Point, Rect, Triangle, coord};

    // records every pixel write so we can assert what each geometry burns
    #[derive(Default)]
    struct Collector {
        cells: Vec<(usize, usize, f64)>,
    }
    impl PixelWriter<f64> for Collector {
        fn write(&mut self, y: usize, x: usize, value: f64, _background: f64) {
            self.cells.push((y, x, value));
        }
    }

    // 10x10, world coord (x, y) -> (row = ymax - y, col = x)
    fn raster_10() -> RasterInfo {
        RasterInfo {
            ncols: 10,
            nrows: 10,
            xmin: 0.0,
            xmax: 10.0,
            ymin: 0.0,
            ymax: 10.0,
            xres: 1.0,
            yres: 1.0,
            epsg: None,
        }
    }

    fn burn(geom: Geometry<f64>) -> Vec<(usize, usize, f64)> {
        let ri = raster_10();
        let mut writer = Collector::default();
        geom.burn::<Standard>(&ri, 1.0, &mut writer, 0.0);
        writer.cells
    }

    #[test]
    fn point_burns_exact_cell() {
        let cells = burn(Geometry::Point(Point::new(2.5, 7.5)));
        assert_eq!(cells, vec![(2, 2, 1.0)]);
    }

    #[test]
    fn multipoint_burns_each_point() {
        let mp = MultiPoint::new(vec![Point::new(1.5, 8.5), Point::new(5.5, 3.5)]);
        let mut cells = burn(Geometry::MultiPoint(mp));
        cells.sort_by_key(|&(y, x, _)| (y, x));
        assert_eq!(cells, vec![(1, 1, 1.0), (6, 5, 1.0)]);
    }

    #[test]
    fn linestring_burns_cells() {
        let ls = LineString::from(vec![(1.0, 5.0), (6.0, 5.0)]);
        let cells = burn(Geometry::LineString(ls));
        assert!(!cells.is_empty(), "horizontal line should burn cells");
        assert!(cells.iter().all(|&(y, _, _)| y == 5));
    }

    #[test]
    fn multilinestring_burns_cells() {
        let mls = MultiLineString::new(vec![
            LineString::from(vec![(1.0, 8.0), (4.0, 8.0)]),
            LineString::from(vec![(1.0, 2.0), (4.0, 2.0)]),
        ]);
        let cells = burn(Geometry::MultiLineString(mls));
        assert!(cells.iter().any(|&(y, _, _)| y == 2));
        assert!(cells.iter().any(|&(y, _, _)| y == 8));
    }

    #[test]
    fn line_burns_diagonal() {
        let line = Line::new(coord! { x: 1.0, y: 1.0 }, coord! { x: 6.0, y: 6.0 });
        let cells = burn(Geometry::Line(line));
        assert!(!cells.is_empty(), "diagonal line should burn cells");
    }

    #[test]
    fn polygon_fills_interior() {
        let poly = Polygon::new(
            LineString::from(vec![(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0), (2.0, 2.0)]),
            vec![],
        );
        let cells = burn(Geometry::Polygon(poly));
        assert!(cells.len() > 4, "square polygon should fill several cells");
    }

    #[test]
    fn multipolygon_fills_both() {
        let p = |x: f64, y: f64| {
            Polygon::new(
                LineString::from(vec![(x, y), (x + 2.0, y), (x + 2.0, y + 2.0), (x, y + 2.0), (x, y)]),
                vec![],
            )
        };
        let mp = MultiPolygon::new(vec![p(1.0, 1.0), p(6.0, 6.0)]);
        let cells = burn(Geometry::MultiPolygon(mp));
        assert!(cells.iter().any(|&(y, _, _)| y >= 6)); // lower polygon
        assert!(cells.iter().any(|&(y, _, _)| y <= 3)); // upper polygon
    }

    #[test]
    fn rect_fills_interior() {
        let rect = Rect::new(coord! { x: 1.0, y: 1.0 }, coord! { x: 5.0, y: 5.0 });
        let cells = burn(Geometry::Rect(rect));
        assert!(cells.len() > 4, "rect should fill several cells");
    }

    #[test]
    fn triangle_fills_interior() {
        let tri = Triangle::new(
            coord! { x: 1.0, y: 1.0 },
            coord! { x: 6.0, y: 1.0 },
            coord! { x: 1.0, y: 6.0 },
        );
        let cells = burn(Geometry::Triangle(tri));
        assert!(!cells.is_empty(), "triangle should fill cells");
    }

    #[test]
    fn geometry_collection_burns_all_members() {
        let gc = GeometryCollection(vec![
            Geometry::Point(Point::new(2.5, 7.5)),
            Geometry::Polygon(Polygon::new(
                LineString::from(vec![(5.0, 1.0), (8.0, 1.0), (8.0, 4.0), (5.0, 4.0), (5.0, 1.0)]),
                vec![],
            )),
        ]);
        let cells = burn(Geometry::GeometryCollection(gc));
        assert!(cells.contains(&(2, 2, 1.0)), "point member should burn");
        assert!(cells.iter().any(|&(y, _, _)| y >= 6), "polygon member should fill");
    }
}
