use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ServerHandler, ServiceExt,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use shacl_rust::{core::ShapesInfo, validation::dataset::ValidationDataset};
use shacl_rust::{
    parse_shapes, rdf::read_graph_from_string, rdf::serialize_graph_to_string, validate,
};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
pub struct ShaclServer {
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for validating RDF data against SHACL shapes")]
struct ValidateGraphsArgs {
    #[schemars(description = "RDF data graph as a string")]
    data_graph: String,
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')")]
    data_format: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
    #[schemars(
        description = "Format of the output report ('text', 'json', or RDF format like 'ttl')"
    )]
    output_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for checking if RDF data conforms to SHACL shapes")]
struct ValidateGraphsConformsArgs {
    #[schemars(description = "RDF data graph as a string")]
    data_graph: String,
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')")]
    data_format: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for validating RDF graph syntax")]
struct LintGraphArgs {
    #[schemars(description = "RDF graph as a string")]
    graph: String,
    #[schemars(description = "Format of the graph (e.g., 'ttl', 'nt', 'jsonld')")]
    format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for parsing SHACL shapes graph")]
struct ParseShapesGraphArgs {
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for validating RDF data against SHACL shapes and returning rich diagnostics"
)]
struct ValidateDiagnosticsArgs {
    #[schemars(description = "RDF data graph as a string")]
    data_graph: String,
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')")]
    data_format: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
    #[schemars(
        description = "Skip the 12 semantic shape-lint rules and only return validation diagnostics (default: false)"
    )]
    #[serde(default)]
    skip_lint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for linting a SHACL shapes graph with the semantic shape-lint rules"
)]
struct LintShaclShapesArgs {
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for explaining a diagnostic code")]
struct ExplainDiagnosticCodeArgs {
    #[schemars(description = "A diagnostic code, e.g. 'V0007' or 'L0003'")]
    code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for tracing why a focus node does or does not conform to SHACL shapes"
)]
struct WhyConformanceArgs {
    #[schemars(description = "RDF data graph as a string")]
    data_graph: String,
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')")]
    data_format: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
    #[schemars(description = "IRI of the focus node to trace, e.g. 'http://example.org/alice'")]
    focus_node: String,
    #[schemars(description = "Optional IRI of a single shape to restrict the trace to")]
    shape: Option<String>,
}

impl Default for ShaclServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl ShaclServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Validate RDF data graph against SHACL shapes graph")]
    async fn validate_graphs(
        &self,
        Parameters(ValidateGraphsArgs {
            data_graph,
            shapes_graph,
            data_format,
            shapes_format,
            output_format,
        }): Parameters<ValidateGraphsArgs>,
    ) -> Result<String, String> {
        let data_graph = read_graph_from_string(&data_graph, &data_format)
            .map_err(|e| format!("Failed to parse data graph: {}", e))?;

        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Failed to parse shapes graph: {}", e))?;

        let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)
            .map_err(|e| format!("Failed to create validation dataset: {}", e))?;

        let shapes = parse_shapes(validation_dataset.shapes_graph())
            .map_err(|e| format!("Failed to parse shapes: {}", e))?;

        let report = validate(&validation_dataset, &shapes);

        let report_string = match output_format.as_str() {
            "json" => report.as_json().to_string(),
            "text" => report.to_string(),
            _ => {
                // Try to parse as RDF format (ttl, nt, nq, rdf, jsonld, trig)
                use oxigraph::io::RdfFormat;
                let rdf_format = RdfFormat::from_extension(output_format.as_str()).ok_or_else(|| {
                    format!(
                        "Unsupported output format: '{}'. Supported: text, json, ttl, nt, nq, rdf, jsonld, trig",
                        output_format
                    )
                })?;

                // Convert validation report to RDF graph
                let report_graph = report.to_graph();

                // Serialize to string
                serialize_graph_to_string(&report_graph, rdf_format)
                    .map_err(|e| format!("Failed to serialize report graph: {}", e))?
            }
        };

        Ok(report_string)
    }

    #[tool(
        description = "Check if RDF data conforms to SHACL shapes (returns only boolean result)"
    )]
    async fn validate_graphs_conforms(
        &self,
        Parameters(ValidateGraphsConformsArgs {
            data_graph,
            shapes_graph,
            data_format,
            shapes_format,
        }): Parameters<ValidateGraphsConformsArgs>,
    ) -> Result<String, String> {
        let data_graph = read_graph_from_string(&data_graph, &data_format)
            .map_err(|e| format!("Failed to parse data graph: {}", e))?;

        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Failed to parse shapes graph: {}", e))?;

        let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)
            .map_err(|e| format!("Failed to create validation dataset: {}", e))?;

        let shapes = parse_shapes(validation_dataset.shapes_graph())
            .map_err(|e| format!("Failed to parse shapes: {}", e))?;

        let report = validate(&validation_dataset, &shapes);

        Ok(json!({ "conforms": *report.get_conforms() }).to_string())
    }

    #[tool(description = "Validate RDF graph syntax")]
    async fn lint_graph(
        &self,
        Parameters(LintGraphArgs { graph, format }): Parameters<LintGraphArgs>,
    ) -> Result<String, String> {
        read_graph_from_string(&graph, &format)
            .map_err(|e| format!("Graph syntax error: {}", e))?;

        Ok(json!({ "valid": true }).to_string())
    }

    #[tool(description = "Parse SHACL shapes graph and return parsed shape information")]
    async fn parse_shapes_graph(
        &self,
        Parameters(ParseShapesGraphArgs {
            shapes_graph,
            shapes_format,
        }): Parameters<ParseShapesGraphArgs>,
    ) -> Result<String, String> {
        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Shapes graph syntax error: {}", e))?;

        let parsed_shapes =
            parse_shapes(&shapes_graph).map_err(|e| format!("SHACL shapes error: {}", e))?;

        Ok(ShapesInfo::new(&parsed_shapes, shapes_graph.len(), true).to_string())
    }

    #[tool(
        description = "Validate RDF data against SHACL shapes and return rich rustc-style diagnostics (lint findings plus violation diagnostics), sorted"
    )]
    async fn validate_diagnostics(
        &self,
        Parameters(ValidateDiagnosticsArgs {
            data_graph,
            shapes_graph,
            data_format,
            shapes_format,
            skip_lint,
        }): Parameters<ValidateDiagnosticsArgs>,
    ) -> Result<String, String> {
        let data_graph = read_graph_from_string(&data_graph, &data_format)
            .map_err(|e| format!("Failed to parse data graph: {}", e))?;

        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Failed to parse shapes graph: {}", e))?;

        let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)
            .map_err(|e| format!("Failed to create validation dataset: {}", e))?;

        let shapes = parse_shapes(validation_dataset.shapes_graph())
            .map_err(|e| format!("Failed to parse shapes: {}", e))?;

        let mut diagnostics = if skip_lint {
            Vec::new()
        } else {
            shacl_rust::diagnostics::lint_shapes(validation_dataset.shapes_graph(), &shapes)
        };

        let report = validate(&validation_dataset, &shapes);
        diagnostics.extend(shacl_rust::diagnostics::from_report(
            &report,
            &validation_dataset,
            &shapes,
        ));
        shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

        let json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        Ok(serde_json::Value::Array(json_array).to_string())
    }

    #[tool(
        description = "Run the 12 semantic shape-lint rules against a SHACL shapes graph and return lint diagnostics"
    )]
    async fn lint_shacl_shapes(
        &self,
        Parameters(LintShaclShapesArgs {
            shapes_graph,
            shapes_format,
        }): Parameters<LintShaclShapesArgs>,
    ) -> Result<String, String> {
        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Shapes graph syntax error: {}", e))?;

        let shapes =
            parse_shapes(&shapes_graph).map_err(|e| format!("SHACL shapes error: {}", e))?;

        let mut diagnostics = shacl_rust::diagnostics::lint_shapes(&shapes_graph, &shapes);
        shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

        let json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        Ok(serde_json::Value::Array(json_array).to_string())
    }

    #[tool(
        description = "Look up a diagnostic code (e.g. 'V0007') in the registry and return its title, spec reference, explanation, and a failing/fixed Turtle example pair"
    )]
    async fn explain_diagnostic_code(
        &self,
        Parameters(ExplainDiagnosticCodeArgs { code }): Parameters<ExplainDiagnosticCodeArgs>,
    ) -> Result<String, String> {
        let entry = shacl_rust::diagnostics::entry(&code)
            .ok_or_else(|| format!("Unknown diagnostic code: {}", code))?;

        Ok(json!({
            "code": entry.code,
            "title": entry.title,
            "component": entry.component,
            "spec_ref": entry.spec_ref,
            "explanation": entry.explanation,
            "failing_example": entry.failing_example,
            "fixed_example": entry.fixed_example,
        })
        .to_string())
    }

    #[tool(
        description = "Trace why a focus node does or does not conform to SHACL shapes, constraint by constraint"
    )]
    async fn why_conformance(
        &self,
        Parameters(WhyConformanceArgs {
            data_graph,
            shapes_graph,
            data_format,
            shapes_format,
            focus_node,
            shape,
        }): Parameters<WhyConformanceArgs>,
    ) -> Result<String, String> {
        let data_graph = read_graph_from_string(&data_graph, &data_format)
            .map_err(|e| format!("Failed to parse data graph: {}", e))?;

        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Failed to parse shapes graph: {}", e))?;

        let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)
            .map_err(|e| format!("Failed to create validation dataset: {}", e))?;

        let shapes = parse_shapes(validation_dataset.shapes_graph())
            .map_err(|e| format!("Failed to parse shapes: {}", e))?;

        let focus_trimmed = trim_angle_brackets(&focus_node);
        let focus_named = oxigraph::model::NamedNode::new(focus_trimmed)
            .map_err(|e| format!("Invalid focus IRI '{}': {}", focus_trimmed, e))?;
        let focus_term = validation_dataset
            .data()
            .canonical_term(oxigraph::model::TermRef::from(focus_named.as_ref()))
            .unwrap_or_else(|| oxigraph::model::TermRef::from(focus_named.as_ref()));

        let shape_named = match &shape {
            Some(s) => {
                let shape_trimmed = trim_angle_brackets(s);
                Some(
                    oxigraph::model::NamedNode::new(shape_trimmed)
                        .map_err(|e| format!("Invalid shape IRI '{}': {}", shape_trimmed, e))?,
                )
            }
            None => None,
        };
        let shape_filter = shape_named
            .as_ref()
            .map(|n| oxigraph::model::NamedOrBlankNodeRef::from(n.as_ref()));

        let diagnostics = shacl_rust::diagnostics::explain_conformance(
            &validation_dataset,
            &shapes,
            focus_term,
            shape_filter,
        );

        let json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        Ok(serde_json::Value::Array(json_array).to_string())
    }
}

/// Trims a single pair of optional surrounding angle brackets (`<...>`) from
/// an IRI argument, mirroring the CLI's and wasm binding's own identically
/// named helper so a caller can pass either `http://example.org/a` or
/// `<http://example.org/a>` (e.g. a `focus_node`/`source_shape` display
/// string copied straight from another tool's diagnostic output, which is
/// always bracket-wrapped for IRIs).
fn trim_angle_brackets(s: &str) -> &str {
    s.trim().trim_start_matches('<').trim_end_matches('>')
}

// Implement the server handler
#[tool_handler]
impl ServerHandler for ShaclServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "SHACL validation server for validating RDF data against SHACL shapes".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// Run the server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    // Create an instance of our counter router
    let shacl_server = ShaclServer::new();
    let shacl_service = shacl_server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    shacl_service.waiting().await?;

    Ok(())
}
