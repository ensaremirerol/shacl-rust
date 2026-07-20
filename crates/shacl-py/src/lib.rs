//! Python bindings for shacl-rust.
//!
//! `validate`/`conforms` take graphs as strings; `validate_file`/
//! `conforms_file` stream from paths or binary file-like objects, so large
//! inputs are never materialized in memory.

use std::io::Read;
use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

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

/// A graph input: a filesystem path or a binary file-like object.
enum GraphSource {
    Path(PathBuf),
    Stream(PyObject),
}

impl GraphSource {
    fn extract(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if value.hasattr("read")? {
            return Ok(GraphSource::Stream(value.clone().unbind()));
        }
        let path: PathBuf = value.extract().map_err(|_| {
            PyValueError::new_err(
                "expected a path (str/os.PathLike) or a binary file-like object with .read()",
            )
        })?;
        Ok(GraphSource::Path(path))
    }

    /// Explicit format, or the path extension; streams have no extension.
    fn resolve_format(&self, explicit: Option<&str>, arg_name: &str) -> PyResult<String> {
        if let Some(format) = explicit {
            return Ok(format.to_string());
        }
        match self {
            GraphSource::Path(path) => path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "cannot infer RDF format from '{}'; pass {arg_name}=",
                        path.display()
                    ))
                }),
            GraphSource::Stream(_) => Err(PyValueError::new_err(format!(
                "file-like inputs require an explicit {arg_name}="
            ))),
        }
    }

    fn into_reader(self) -> Result<Box<dyn Read + Send>, ShaclError> {
        match self {
            GraphSource::Path(path) => {
                let file = std::fs::File::open(&path).map_err(|e| {
                    ShaclError::Io(format!("Failed to open '{}': {}", path.display(), e))
                })?;
                Ok(Box::new(file))
            }
            GraphSource::Stream(obj) => Ok(Box::new(PyReader { obj })),
        }
    }
}

/// `std::io::Read` over a Python binary file-like object. Reacquires the GIL
/// per chunk, so the surrounding validation can run with the GIL released.
struct PyReader {
    obj: PyObject,
}

impl Read for PyReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Python::with_gil(|py| {
            let chunk = self
                .obj
                .call_method1(py, "read", (buf.len(),))
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let chunk = chunk.bind(py);
            if chunk.is_none() {
                return Ok(0);
            }
            let bytes = chunk.downcast::<PyBytes>().map_err(|_| {
                std::io::Error::other(
                    "file-like object must be opened in binary mode (read() must return bytes)",
                )
            })?;
            let data = bytes.as_bytes();
            let n = data.len().min(buf.len());
            buf[..n].copy_from_slice(&data[..n]);
            Ok(n)
        })
    }
}

fn build_dataset(
    data: impl Read,
    data_format: &str,
    shapes: impl Read,
    shapes_format: &str,
    experimental_index: bool,
) -> Result<ValidationDataset, ShaclError> {
    let shapes_graph = rdf::read_graph_from_reader(shapes, shapes_format)?;

    if experimental_index {
        // Stream parser output straight into the index; first error wins.
        let mut first_err: Option<ShaclError> = None;
        let triples = rdf::parse_triples_from_reader(data, data_format)?;
        let triples = triples.filter_map(|result| match result {
            Ok(triple) => Some(triple),
            Err(e) => {
                first_err.get_or_insert(e);
                None
            }
        });
        let dataset =
            ValidationDataset::from_triples_with_experimental_index(triples, shapes_graph)?;
        if let Some(e) = first_err {
            return Err(e);
        }
        Ok(dataset)
    } else {
        let data_graph = rdf::read_graph_from_reader(data, data_format)?;
        ValidationDataset::from_graphs(data_graph, shapes_graph)
    }
}

fn run_validation(dataset: &ValidationDataset) -> Result<serde_json::Value, ShaclError> {
    let parsed_shapes = parse_shapes(dataset.shapes_graph())?;
    let report = shacl_rust_core::validate(dataset, &parsed_shapes);
    Ok(report.as_json())
}

fn validate_strings(
    data: &str,
    shapes: &str,
    data_format: &str,
    shapes_format: &str,
    experimental_index: bool,
) -> Result<serde_json::Value, ShaclError> {
    let dataset = build_dataset(
        data.as_bytes(),
        data_format,
        shapes.as_bytes(),
        shapes_format,
        experimental_index,
    )?;
    run_validation(&dataset)
}

fn validate_sources(
    py: Python<'_>,
    data: GraphSource,
    shapes: GraphSource,
    data_format: &str,
    shapes_format: &str,
    experimental_index: bool,
) -> PyResult<serde_json::Value> {
    py.allow_threads(|| {
        let dataset = build_dataset(
            data.into_reader()?,
            data_format,
            shapes.into_reader()?,
            shapes_format,
            experimental_index,
        )?;
        run_validation(&dataset)
    })
    .map_err(shacl_err)
}

/// Validate RDF data against SHACL shapes; both graphs are strings.
///
/// Formats accept the same names as the CLI ("turtle"/"ttl", "nt", "nq",
/// "rdf", "jsonld", "trig"). Returns the validation report as a dict with a
/// boolean "conforms" key and a "results" list. Set experimental_index=True
/// to load the data graph into the experimental interned index (faster on
/// large graphs).
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
            validate_strings(data, shapes, data_format, shapes_format, experimental_index)
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
            validate_strings(data, shapes, data_format, shapes_format, experimental_index)
        })
        .map_err(shacl_err)?;
    Ok(json
        .get("conforms")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

/// Validate by streaming from paths or binary file-like objects.
///
/// `data` and `shapes` each accept a path (str/os.PathLike) or any object
/// with a binary .read() method (open(..., "rb"), gzip.open, io.BytesIO,
/// sockets). The serialized text is never fully held in memory. Formats are
/// inferred from path extensions; file-like inputs require data_format= /
/// shapes_format=.
#[pyfunction]
#[pyo3(signature = (data, shapes, *, data_format = None, shapes_format = None, experimental_index = false))]
fn validate_file(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    shapes: &Bound<'_, PyAny>,
    data_format: Option<&str>,
    shapes_format: Option<&str>,
    experimental_index: bool,
) -> PyResult<PyObject> {
    let data = GraphSource::extract(data)?;
    let shapes = GraphSource::extract(shapes)?;
    let data_format = data.resolve_format(data_format, "data_format")?;
    let shapes_format = shapes.resolve_format(shapes_format, "shapes_format")?;
    let json = validate_sources(
        py,
        data,
        shapes,
        &data_format,
        &shapes_format,
        experimental_index,
    )?;
    json_to_py(py, &json)
}

/// Streaming variant of `conforms`; accepts the same inputs as
/// `validate_file`.
#[pyfunction]
#[pyo3(signature = (data, shapes, *, data_format = None, shapes_format = None, experimental_index = false))]
fn conforms_file(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    shapes: &Bound<'_, PyAny>,
    data_format: Option<&str>,
    shapes_format: Option<&str>,
    experimental_index: bool,
) -> PyResult<bool> {
    let data = GraphSource::extract(data)?;
    let shapes = GraphSource::extract(shapes)?;
    let data_format = data.resolve_format(data_format, "data_format")?;
    let shapes_format = shapes.resolve_format(shapes_format, "shapes_format")?;
    let json = validate_sources(
        py,
        data,
        shapes,
        &data_format,
        &shapes_format,
        experimental_index,
    )?;
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
    m.add_function(wrap_pyfunction!(validate_file, m)?)?;
    m.add_function(wrap_pyfunction!(conforms_file, m)?)?;
    Ok(())
}
