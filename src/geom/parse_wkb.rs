/*
Zero-copy deserialization of WKB geometries from Arrow Table
 */

use geo_types::Geometry;
use geozero::wkb::{WkbDialect, FromWkb};
use std::io::Cursor;

fn parse_wkb_goemetry(wkb: &[u8]) -> Result<Geometry<f64>, geozero::error::GeozeroError> {
    let mut reader = Cursor::new(wkb);
    FromWkb::from_wkb(&mut reader, WkbDialect::Wkb)
}