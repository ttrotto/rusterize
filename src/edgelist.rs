/*
Build edge list from polygon or multipolygon
*/

mod edge;

use pyo3::prelude::*;

#[pyfunction]
fn edgelist()