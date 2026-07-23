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
pub fn validate_graphs_diagnostics(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
    skip_lint: bool,
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

    let mut diagnostics = if skip_lint {
        Vec::new()
    } else {
        shacl_rust::diagnostics::lint_shapes(validation_dataset.shapes_graph(), &parsed_shapes)
    };

    let report = validate(&validation_dataset, &parsed_shapes);
    diagnostics.extend(shacl_rust::diagnostics::from_report(
        &report,
        &validation_dataset,
        &parsed_shapes,
    ));
    shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

    Ok(shacl_rust::diagnostics::render_text(&diagnostics, false))
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

#[wasm_bindgen]
pub fn validate_diagnostics_json(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
    skip_lint: bool,
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

    let mut diagnostics = if skip_lint {
        Vec::new()
    } else {
        shacl_rust::diagnostics::lint_shapes(validation_dataset.shapes_graph(), &parsed_shapes)
    };

    let report = validate(&validation_dataset, &parsed_shapes);
    diagnostics.extend(shacl_rust::diagnostics::from_report(
        &report,
        &validation_dataset,
        &parsed_shapes,
    ));
    shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

    let json_array: Vec<serde_json::Value> = diagnostics
        .iter()
        .map(shacl_rust::diagnostics::diagnostic_to_json)
        .collect();
    serde_json::to_string(&json_array)
        .map_err(|e| to_js_error(format!("Failed to serialize diagnostics: {}", e)))
}

#[wasm_bindgen]
pub fn shape_target_nodes_json(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
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

    let entries = shacl_rust::diagnostics::shape_target_nodes(&validation_dataset, &parsed_shapes);

    let json_array: Vec<serde_json::Value> = entries
        .into_iter()
        .map(|(shape, nodes)| {
            let targets: Vec<serde_json::Value> = nodes
                .into_iter()
                .map(|node| {
                    let term_kind = if node.starts_with("_:") {
                        "blank"
                    } else {
                        "iri"
                    };
                    serde_json::json!({ "node": node, "term_kind": term_kind })
                })
                .collect();
            serde_json::json!({ "shape": shape, "targets": targets })
        })
        .collect();

    serde_json::to_string(&json_array)
        .map_err(|e| to_js_error(format!("Failed to serialize shape target nodes: {}", e)))
}

#[wasm_bindgen]
pub fn explain_code_json(code: &str) -> Result<String, JsValue> {
    let entry = shacl_rust::diagnostics::entry(code)
        .ok_or_else(|| to_js_error(format!("Unknown diagnostic code: {}", code)))?;

    let json = serde_json::json!({
        "code": entry.code,
        "title": entry.title,
        "component": entry.component,
        "spec_ref": entry.spec_ref,
        "explanation": entry.explanation,
        "failing_example": entry.failing_example,
        "fixed_example": entry.fixed_example,
    });

    serde_json::to_string(&json)
        .map_err(|e| to_js_error(format!("Failed to serialize registry entry: {}", e)))
}

/// Trims a single pair of optional surrounding angle brackets (`<...>`) from
/// an IRI argument, mirroring the CLI's `--focus`/`--shape` parsing
/// (`crates/shacl-cli/src/main.rs`'s `trim_angle_brackets`) so the web demo
/// can pass a `Diagnostic.focus_node`/`shape_target_nodes_json` display
/// string (always bracket-wrapped for IRIs) straight through.
fn trim_angle_brackets(s: &str) -> &str {
    s.trim().trim_start_matches('<').trim_end_matches('>')
}

#[wasm_bindgen]
pub fn why_json(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
    focus_iri: &str,
    shape_iri: &str,
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

    let focus_trimmed = trim_angle_brackets(focus_iri);
    let focus_node = oxigraph::model::NamedNode::new(focus_trimmed)
        .map_err(|e| to_js_error(format!("Invalid focus IRI '{}': {}", focus_trimmed, e)))?;
    let focus_term = validation_dataset
        .data()
        .canonical_term(oxigraph::model::TermRef::from(focus_node.as_ref()))
        .unwrap_or_else(|| oxigraph::model::TermRef::from(focus_node.as_ref()));

    let shape_node =
        if shape_iri.is_empty() {
            None
        } else {
            let shape_trimmed = trim_angle_brackets(shape_iri);
            Some(oxigraph::model::NamedNode::new(shape_trimmed).map_err(|e| {
                to_js_error(format!("Invalid shape IRI '{}': {}", shape_trimmed, e))
            })?)
        };
    let shape_filter = shape_node
        .as_ref()
        .map(|n| oxigraph::model::NamedOrBlankNodeRef::from(n.as_ref()));

    let diags = shacl_rust::diagnostics::explain_conformance(
        &validation_dataset,
        &parsed_shapes,
        focus_term,
        shape_filter,
    );

    let json_array: Vec<serde_json::Value> = diags
        .iter()
        .map(shacl_rust::diagnostics::diagnostic_to_json)
        .collect();
    serde_json::to_string(&json_array)
        .map_err(|e| to_js_error(format!("Failed to serialize why trace: {}", e)))
}
