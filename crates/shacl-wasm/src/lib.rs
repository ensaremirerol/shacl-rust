use wasm_bindgen::prelude::*;

use shacl_rust::validation::report::{ValidationReport, ValidationResult};
use shacl_rust::{
    parse_shapes, rdf::read_graph_from_string, rdf::serialize_graph_to_string, validate,
};

use oxigraph::io::RdfFormat;

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

fn validation_result_to_json(result: &ValidationResult<'_>) -> serde_json::Value {
    use serde_json::json;

    json!({
        "focusNode": result.focus_node.to_string(),
        "sourceShape": result.source_shape.to_string(),
        "sourceConstraintComponent": result.source_constraint_component.map(|c| c.to_string()),
        "severity": result.severity.to_string(),
        "resultPath": result.result_path.as_ref().map(|p| p.to_string()),
        "value": result.value.map(|v| v.to_string()),
        "messages": result.messages,
        "details": result.details.iter().map(validation_result_to_json).collect::<Vec<_>>(),
    })
}

fn validation_report_to_json(report: &ValidationReport<'_>) -> serde_json::Value {
    serde_json::json!({
        "conforms": report.conforms,
        "results": report.results.iter().map(validation_result_to_json).collect::<Vec<_>>(),
    })
}

#[wasm_bindgen]
pub fn validate_graphs_json(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
) -> Result<String, JsValue> {
    let data = read_graph_from_string(data_graph, data_format)
        .map_err(|e| to_js_error(format!("Failed to parse data graph: {}", e)))?;
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Failed to parse shapes graph: {}", e)))?;

    let parsed_shapes = parse_shapes(&shapes)
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    let report = validate(&data, &parsed_shapes);

    let json_report = validation_report_to_json(&report);

    serde_json::to_string(&json_report)
        .map_err(|e| to_js_error(format!("Failed to serialize validation report: {}", e)))
}

#[wasm_bindgen]
pub fn validate_graphs_output(
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

    let parsed_shapes = parse_shapes(&shapes)
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    let report = validate(&data, &parsed_shapes);

    match output_format.to_ascii_lowercase().as_str() {
        "text" => Ok(report.to_string()),
        "json" => {
            let json_report = validation_report_to_json(&report);
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
pub fn validate_graphs_all_formats(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
    graph_format: &str,
) -> Result<String, JsValue> {
    let data = read_graph_from_string(data_graph, data_format)
        .map_err(|e| to_js_error(format!("Failed to parse data graph: {}", e)))?;
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Failed to parse shapes graph: {}", e)))?;

    let parsed_shapes = parse_shapes(&shapes)
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    let report = validate(&data, &parsed_shapes);

    let rdf_format =
        RdfFormat::from_extension(&graph_format.to_ascii_lowercase()).ok_or_else(|| {
            to_js_error(format!(
                "Unsupported graph format: '{}'. Use an RDF extension like ttl/nt/nq/rdf/jsonld/trig",
                graph_format
            ))
        })?;

    let graph_output = serialize_graph_to_string(&report.to_graph(), rdf_format)
        .map_err(|e| to_js_error(format!("Failed to serialize report graph: {}", e)))?;

    let payload = serde_json::json!({
        "text": report.to_string(),
        "json": validation_report_to_json(&report),
        "graph": graph_output,
        "graphFormat": graph_format,
    });

    serde_json::to_string(&payload)
        .map_err(|e| to_js_error(format!("Failed to serialize output payload: {}", e)))
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

    let parsed_shapes = parse_shapes(&shapes)
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    Ok(validate(&data, &parsed_shapes).conforms)
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
