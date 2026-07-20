//! Python bindings for shacl-rust.
//!
//! Exposes a single `validate` function returning the validation report as a
//! Python dict (same shape as the library's JSON report), plus a `conforms`
//! convenience wrapper.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use shacl_rust_core::validation::dataset::ValidationDataset;
use shacl_rust_core::{parse_shapes, rdf, ShaclError};

fn shacl_err(e: ShaclError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    Ok(match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_pyobject(py)?.to_owned().unbind().into(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py)?.unbind().into()
            } else {
                n.as_f64()
                    .unwrap_or(f64::NAN)
                    .into_pyobject(py)?
                    .unbind()
                    .into()
            }
        }
        serde_json::Value::String(s) => s.into_pyobject(py)?.unbind().into(),
        serde_json::Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            list.unbind().into()
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, item) in map {
                dict.set_item(key, json_to_py(py, item)?)?;
            }
            dict.unbind().into()
        }
    })
}

fn run_validation(
    data: &str,
    shapes: &str,
    data_format: &str,
    shapes_format: &str,
    experimental_index: bool,
) -> Result<serde_json::Value, ShaclError> {
    let shapes_graph = rdf::read_graph_from_string(shapes, shapes_format)?;

    let dataset = if experimental_index {
        let triples =
            rdf::parse_triples_from_string(data, data_format)?.collect::<Result<Vec<_>, _>>()?;
        ValidationDataset::from_triples_with_experimental_index(triples, shapes_graph)?
    } else {
        let data_graph = rdf::read_graph_from_string(data, data_format)?;
        ValidationDataset::from_graphs(data_graph, shapes_graph)?
    };

    let parsed_shapes = parse_shapes(dataset.shapes_graph())?;
    let report = shacl_rust_core::validate(&dataset, &parsed_shapes);
    Ok(report.as_json())
}

/// Validate RDF data against SHACL shapes.
///
/// Both graphs are passed as strings; formats accept the same names as the
/// CLI ("turtle"/"ttl", "nt", "nq", "rdf", "jsonld", "trig"). Returns the
/// validation report as a dict with at least a boolean "conforms" key and a
/// "results" list. Set experimental_index=True to load the data graph into
/// the experimental interned index (faster on large graphs).
#[pyfunction]
#[pyo3(signature = (data, shapes, *, data_format = "turtle", shapes_format = "turtle", experimental_index = false))]
fn validate(
    py: Python<'_>,
    data: &str,
    shapes: &str,
    data_format: &str,
    shapes_format: &str,
    experimental_index: bool,
) -> PyResult<PyObject> {
    let json = py
        .allow_threads(|| {
            run_validation(data, shapes, data_format, shapes_format, experimental_index)
        })
        .map_err(shacl_err)?;
    json_to_py(py, &json)
}

/// Returns True when the data graph conforms to the shapes graph.
#[pyfunction]
#[pyo3(signature = (data, shapes, *, data_format = "turtle", shapes_format = "turtle", experimental_index = false))]
fn conforms(
    py: Python<'_>,
    data: &str,
    shapes: &str,
    data_format: &str,
    shapes_format: &str,
    experimental_index: bool,
) -> PyResult<bool> {
    let json = py
        .allow_threads(|| {
            run_validation(data, shapes, data_format, shapes_format, experimental_index)
        })
        .map_err(shacl_err)?;
    Ok(json
        .get("conforms")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

#[pymodule]
fn shacl_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(conforms, m)?)?;
    Ok(())
}
