use geo::Geometry;
use geo_traits::to_geo::ToGeoGeometry;
use savvy::{ListSexp, Result as SavvyResult, TypedSexp, savvy_err};
use wkb::reader::read_wkb;

/// Parse geometries from WKB. Silently drops if a geometry cannot be parsed.
pub(crate) fn parse_geometry(geoms: ListSexp) -> SavvyResult<Vec<Geometry<f64>>> {
    if geoms.is_empty() {
        return Err(savvy_err!("Geometry list is empty."));
    }

    let parsed = geoms
        .values_iter()
        .filter_map(|sexp| {
            let TypedSexp::Raw(bytes) = sexp.into_typed() else {
                return None;
            };
            read_wkb(bytes.as_slice()).ok()?.try_to_geometry()
        })
        .collect::<Vec<Geometry<f64>>>();

    Ok(parsed)
}
