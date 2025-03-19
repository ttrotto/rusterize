/*
Extract geometries from geoarrow::Table
*/

use geo_types::Geometry;
use geoarrow::array::GeometryCollectionArray;
use geoarrow::table::Table;
use geo_traits::to_geo::ToGeoGeometryCollection;
use geoarrow::trait_::ArrayAccessor;

fn to_geo(table: Table) -> Vec<Geometry> {
    // collect geometries
    let mut geovec: Vec<Geometry> = Vec::new();
    
    // Table -> ChunkedArray -> GeometryCollectionArray
    let chunked_array = table.geometry_column(None).expect("No geometry column found!");
    let geom_array = chunked_array
        .as_any()
        .downcast_ref::<GeometryCollectionArray>()
        .unwrap();
        
    // push geometries out of the collection into a unified vector
    geovec.extend(
        geom_array
            .iter()
            .filter_map(|geom_collection| {
                geom_collection
                    .and_then(|gc| ToGeoGeometryCollection::try_to_geometry_collection(&gc))
            })
            .flat_map(|gcollection| gcollection.0.into_iter())
    );
    geovec
}