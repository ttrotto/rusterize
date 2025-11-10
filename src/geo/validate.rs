/* Check unsupported geometries and adjust bounding box if necessary */

use crate::geo::raster::RasterInfo;
use geo::BoundingRect;
use geo_types::{Geometry, Rect, coord};
use polars::prelude::*;
use pyo3::prelude::*;

// https://github.com/georust/geo/blob/main/geo/src/algorithm/bounding_rect.rs#L186
fn bounding_rect(geometry: &[Geometry]) -> Option<Rect> {
    geometry.iter().fold(None, |acc, next| {
        let next_bounding_rect = next.bounding_rect();

        // enlarge bounding rectangle if necessary
        match (acc, next_bounding_rect) {
            (None, None) => None,
            (Some(r), None) | (None, Some(r)) => Some(r),
            (Some(r1), Some(r2)) => Some(bounding_rect_merge(r1, r2)),
        }
    })
}

// https://github.com/georust/geo/blob/main/geo/src/algorithm/bounding_rect.rs#L200
fn bounding_rect_merge(a: Rect, b: Rect) -> Rect {
    Rect::new(
        coord! {
            x: a.min().x.min(b.min().x),
            y: a.min().y.min(b.min().y),
        },
        coord! {
            x: a.max().x.max(b.max().x),
            y: a.max().y.max(b.max().y),
        },
    )
}

pub fn validate_geometries(
    mut geometry: Vec<Geometry>,
    mut df: Option<DataFrame>,
    raster_info: &mut RasterInfo,
) -> (Vec<Geometry>, Option<DataFrame>) {
    // check if any bad geometry
    let mut good_geom: Vec<bool> = Vec::with_capacity(geometry.len());
    let mut has_invalid = 0u32;
    for geom in &geometry {
        let valid = matches!(
            geom,
            &Geometry::Polygon(_)
                | &Geometry::MultiPolygon(_)
                | &Geometry::LineString(_)
                | &Geometry::MultiLineString(_)
                | &Geometry::GeometryCollection(_)
        );
        if !valid {
            has_invalid += 1;
        }
        good_geom.push(valid);
    }

    if has_invalid > 0 {
        // issue warning if bad geometries
        Python::with_gil(|py| {
            let warnings = Python::import(py, "warnings").unwrap();
            warnings
                .call_method1(
                    "warn",
                    (format!(
                        "Detected {has_invalid} unsupported geometries, will be dropped."
                    ),),
                )
                .unwrap();
        });

        // retain only good geometries
        let mut iter = good_geom.iter();
        geometry.retain(|_| *iter.next().unwrap());

        // early stop if no supported geometries are left
        if geometry.is_empty() {
            panic!("There are no supported geometries left to rasterize")
        }

        // retain dataframe rows accordingly
        if let Some(inner_df) = df {
            df = inner_df
                .filter(&BooleanChunked::from_iter_values(
                    "good_geom".into(),
                    good_geom.into_iter(),
                ))
                .ok();
        }

        // update RasterInfo spatial properties
        if !raster_info.has_extent {
            let bbox = bounding_rect(&geometry).unwrap();
            raster_info.update_bounds(bbox);
        }
    }

    // update RasterInfo spatial properties
    raster_info.update_dims();
    (geometry, df)
}
