use wasm_bindgen::prelude::*;

use shacl_rust::{
    parse_shapes, rdf::read_graph_from_string, rdf::serialize_graph_to_string, validate,
};

use oxigraph::io::RdfFormat;

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[wasm_bindgen]
pub fn validate_graphs(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
    output_format: &str,
) -> Result<String, JsValue> {
    let data = read_graph_from_string(data_graph, data_format)
        .map_err(|e| to_js_error(format!("Failed to parse data graph: {}", e)))?;
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Failed to parse shapes graph: {}", e)))?;

    let validation_dataset =
        shacl_rust::validation::dataset::ValidationDataset::from_graphs(data, shapes)
            .map_err(|e| to_js_error(format!("Failed to create validation dataset: {}", e)))?;

    let parsed_shapes = parse_shapes(validation_dataset.shapes_graph())
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    let report = validate(&validation_dataset, &parsed_shapes);

    match output_format.to_ascii_lowercase().as_str() {
        "text" => Ok(report.to_string()),
        "json" => {
            let json_report = report.as_json();
            serde_json::to_string(&json_report)
                .map_err(|e| to_js_error(format!("Failed to serialize validation report: {}", e)))
        }
        format_extension => {
            let rdf_format = RdfFormat::from_extension(format_extension).ok_or_else(|| {
                to_js_error(format!(
                    "Unsupported output format: '{}'. Use text, json, or an RDF extension like ttl/nt/nq/rdf/jsonld/trig",
                    output_format
                ))
            })?;

            let report_graph = report.to_graph();
            serialize_graph_to_string(&report_graph, rdf_format)
                .map_err(|e| to_js_error(format!("Failed to serialize report graph: {}", e)))
        }
    }
}

#[wasm_bindgen]
pub fn validate_graphs_conforms(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
) -> Result<bool, JsValue> {
    let data = read_graph_from_string(data_graph, data_format)
        .map_err(|e| to_js_error(format!("Failed to parse data graph: {}", e)))?;
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Failed to parse shapes graph: {}", e)))?;

    let validation_dataset =
        shacl_rust::validation::dataset::ValidationDataset::from_graphs(data, shapes)
            .map_err(|e| to_js_error(format!("Failed to create validation dataset: {}", e)))?;

    let parsed_shapes = parse_shapes(validation_dataset.shapes_graph())
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    Ok(*validate(&validation_dataset, &parsed_shapes).get_conforms())
}

#[wasm_bindgen]
pub fn lint_data_graph(data_graph: &str, data_format: &str) -> Result<(), JsValue> {
    read_graph_from_string(data_graph, data_format)
        .map(|_| ())
        .map_err(|e| to_js_error(format!("Data graph syntax error: {}", e)))
}

#[wasm_bindgen]
pub fn lint_shapes_graph(shapes_graph: &str, shapes_format: &str) -> Result<(), JsValue> {
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Shapes graph syntax error: {}", e)))?;

    parse_shapes(&shapes)
        .map(|_| ())
        .map_err(|e| to_js_error(format!("SHACL shapes error: {}", e)))
}
