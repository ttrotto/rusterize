/*
Check unsupported geometries and adjust bounding box if necessary.
 */
use std::time::Instant;
use crate::structs::raster::RasterInfo;
use geo::BoundingRect;
use geo_types::{coord, Geometry, Rect};
use polars::prelude::*;
use pyo3::prelude::*;
use pyo3::types::PyModule;

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
    let start = Instant::now();
    let mut good_geom: Vec<bool> = Vec::with_capacity(geometry.len());
    let mut has_invalid = false;
    for geom in &geometry {
        let valid = matches!(
            geom,
            &Geometry::Polygon(_)
                | &Geometry::MultiPolygon(_)
                | &Geometry::LineString(_)
                | &Geometry::MultiLineString(_)
        );
        if !valid {
            has_invalid = true;
        }
        good_geom.push(valid);
    }
    println!("Elapsed time has_invalid: {:?}", start.elapsed());
    

    if has_invalid {
        // issue warning if bad geometries
        Python::with_gil(|py| {
            let warnings = PyModule::import_bound(py, "warnings").unwrap();
            warnings
                .call_method1(
                    "warn",
                    ("Detected unsupported geometries, will be dropped.",),
                )
                .unwrap();
        });

        // retain only good geometries
        let mut iter = good_geom.iter();
        geometry.retain(|_| *iter.next().unwrap());

        // retain dataframe rows accordingly
        if let Some(inner_df) = df {
            df = inner_df
                .filter(&BooleanChunked::from_iter_values(
                    PlSmallStr::from("good_geom"),
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
