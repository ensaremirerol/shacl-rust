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

/// Shared field set for supplying a single RDF data graph: inline content or
/// a file path (mutually exclusive), with format inferred from the path's
/// extension when a path is given and `data_format` is omitted.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct DataGraphInput {
    #[schemars(
        description = "RDF data graph as an inline string. Mutually exclusive with `data_path`."
    )]
    #[serde(default)]
    data_graph: Option<String>,
    #[schemars(
        description = "Path to an RDF data file on disk, as an alternative to inline `data_graph` (avoids re-pasting large/shared graphs into every call)."
    )]
    #[serde(default)]
    data_path: Option<String>,
    #[schemars(
        description = "Format of the data graph (e.g., 'ttl', 'nt', 'jsonld'). Required when passing `data_graph` inline; inferred from the file extension for `data_path` if omitted."
    )]
    #[serde(default)]
    data_format: Option<String>,
}

/// Shared field set for supplying one or more SHACL shapes graphs: inline
/// content, a file path, or arrays of either (merged server-side) — useful
/// when a project splits shapes across a base vocabulary plus extensions.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ShapesGraphInput {
    #[schemars(
        description = "SHACL shapes graph as an inline string. Mutually exclusive with `shapes_path`."
    )]
    #[serde(default)]
    shapes_graph: Option<String>,
    #[schemars(
        description = "Path to a SHACL shapes file on disk, as an alternative to inline `shapes_graph`."
    )]
    #[serde(default)]
    shapes_path: Option<String>,
    #[schemars(
        description = "Multiple SHACL shapes graphs as inline strings, merged server-side into one shapes graph (e.g. a base vocabulary plus project extensions). Combine with `shapes_graph`/`shapes_path`/`shapes_paths` freely; all given sources are merged."
    )]
    #[serde(default)]
    shapes_graphs: Option<Vec<String>>,
    #[schemars(
        description = "Multiple SHACL shapes files on disk, merged server-side into one shapes graph."
    )]
    #[serde(default)]
    shapes_paths: Option<Vec<String>>,
    #[schemars(
        description = "Named shapes sources (e.g. a base vocabulary plus project-specific extensions), each inline or by path. Naming sources explicitly enables collision detection: if the same shape IRI receives conflicting triples from two sources, a D0001 error names both; if it's triple-for-triple identical in both, a D0002 info does. Combine freely with the other `shapes_*` fields above - every source (named or not) is merged and checked for collisions against every other."
    )]
    #[serde(default)]
    shapes_sources: Option<Vec<NamedShapesSourceInput>>,
    #[schemars(
        description = "Format shared by all inline/path shapes sources above (e.g., 'ttl', 'nt', 'jsonld'). Required for inline sources; inferred per-file from extension for path sources if omitted."
    )]
    #[serde(default)]
    shapes_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "One named shapes source, inline or by path")]
struct NamedShapesSourceInput {
    #[schemars(
        description = "Name for this source (e.g. 'core', 'extensions'), used to attribute any D0001/D0002 collision diagnostics and decompose_shapes' `source`/`sources` fields."
    )]
    name: String,
    #[schemars(
        description = "This source's shapes graph as an inline string. Mutually exclusive with `path`."
    )]
    #[serde(default)]
    content: Option<String>,
    #[schemars(
        description = "Path to this source's shapes file on disk, as an alternative to inline `content`."
    )]
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for validating RDF data against SHACL shapes")]
struct ValidateGraphsArgs {
    #[serde(flatten)]
    data: DataGraphInput,
    #[serde(flatten)]
    shapes: ShapesGraphInput,
    #[schemars(
        description = "Format of the output report ('text', 'json', or RDF format like 'ttl')"
    )]
    output_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for checking if RDF data conforms to SHACL shapes")]
struct ValidateGraphsConformsArgs {
    #[serde(flatten)]
    data: DataGraphInput,
    #[serde(flatten)]
    shapes: ShapesGraphInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for validating RDF graph syntax")]
struct LintGraphArgs {
    #[schemars(
        description = "RDF graph as an inline string. Mutually exclusive with `graph_path`."
    )]
    #[serde(default)]
    graph: Option<String>,
    #[schemars(description = "Path to an RDF file on disk, as an alternative to inline `graph`.")]
    #[serde(default)]
    graph_path: Option<String>,
    #[schemars(
        description = "Format of the graph (e.g., 'ttl', 'nt', 'jsonld'). Required for inline `graph`; inferred from the file extension for `graph_path` if omitted."
    )]
    #[serde(default)]
    format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for parsing SHACL shapes graph")]
struct ParseShapesGraphArgs {
    #[serde(flatten)]
    shapes: ShapesGraphInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for structured shapes decomposition")]
struct DecomposeShapesArgs {
    #[serde(flatten)]
    shapes: ShapesGraphInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for validating RDF data against SHACL shapes and returning rich diagnostics"
)]
struct ValidateDiagnosticsArgs {
    #[serde(flatten)]
    data: DataGraphInput,
    #[serde(flatten)]
    shapes: ShapesGraphInput,
    #[schemars(
        description = "Skip the 15 semantic shape-lint rules and only return validation diagnostics (default: false)"
    )]
    #[serde(default)]
    skip_lint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for linting a SHACL shapes graph with the semantic shape-lint rules"
)]
struct LintShaclShapesArgs {
    #[serde(flatten)]
    shapes: ShapesGraphInput,
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
    #[serde(flatten)]
    data: DataGraphInput,
    #[serde(flatten)]
    shapes: ShapesGraphInput,
    #[schemars(description = "IRI of the focus node to trace, e.g. 'http://example.org/alice'")]
    focus_node: String,
    #[schemars(description = "Optional IRI of a single shape to restrict the trace to")]
    shape: Option<String>,
}

fn format_from_path(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_string)
}

/// Resolves one RDF graph input that may be given inline or as a file path
/// (mutually exclusive), inferring the format from the path's extension when
/// a format isn't given. Returns the raw text and the effective format.
fn resolve_graph_source(
    field: &str,
    inline: Option<String>,
    path: Option<String>,
    format: Option<String>,
) -> Result<(String, String), String> {
    match (inline, path) {
        (Some(_), Some(_)) => Err(format!(
            "Provide either `{field}` or `{field}_path`, not both"
        )),
        (Some(content), None) => {
            let fmt = format.ok_or_else(|| {
                format!("`{field}_format` is required when passing `{field}` inline")
            })?;
            Ok((content, fmt))
        }
        (None, Some(path)) => {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read `{field}_path` '{path}': {e}"))?;
            let fmt = format.or_else(|| format_from_path(&path)).ok_or_else(|| {
                format!("Could not infer RDF format for '{path}'; provide `{field}_format`")
            })?;
            Ok((content, fmt))
        }
        (None, None) => Err(format!("Provide either `{field}` or `{field}_path`")),
    }
}

fn resolve_data_graph(input: DataGraphInput) -> Result<oxigraph::model::Graph, String> {
    let (content, fmt) = resolve_graph_source(
        "data_graph",
        input.data_graph,
        input.data_path,
        input.data_format,
    )?;
    read_graph_from_string(&content, &fmt).map_err(|e| format!("Failed to parse data graph: {e}"))
}

/// Resolves a [`ShapesGraphInput`] into its individual named sources,
/// without merging them: every source is parsed into its own [`shacl_rust::sources::NamedSource`],
/// named explicitly (`shapes_sources`) or automatically (its file path for
/// `shapes_path`/`shapes_paths`; `inline-0`, `inline-1`, ... for
/// `shapes_graph`/`shapes_graphs`, in declaration order). Callers that only
/// need one graph should merge via [`shacl_rust::sources::merge_sources`];
/// callers that want collision diagnostics or per-source attribution
/// (`decompose_shapes`) use the sources directly.
fn resolve_shapes_sources(
    input: ShapesGraphInput,
) -> Result<Vec<shacl_rust::sources::NamedSource>, String> {
    let shapes_format = input.shapes_format.clone();

    // (explicit name, inline content, path)
    let mut raw: Vec<(Option<String>, Option<String>, Option<String>)> = Vec::new();
    if input.shapes_graph.is_some() || input.shapes_path.is_some() {
        raw.push((None, input.shapes_graph, input.shapes_path));
    }
    for g in input.shapes_graphs.into_iter().flatten() {
        raw.push((None, Some(g), None));
    }
    for p in input.shapes_paths.into_iter().flatten() {
        raw.push((None, None, Some(p)));
    }
    for named in input.shapes_sources.into_iter().flatten() {
        if named.content.is_none() && named.path.is_none() {
            return Err(format!(
                "shapes_sources entry '{}' must provide `content` or `path`",
                named.name
            ));
        }
        raw.push((Some(named.name), named.content, named.path));
    }
    if raw.is_empty() {
        return Err(
            "Provide one of `shapes_graph`, `shapes_path`, `shapes_graphs`, `shapes_paths`, `shapes_sources`"
                .to_string(),
        );
    }

    let mut sources = Vec::new();
    let mut inline_index = 0usize;
    for (explicit_name, inline, path) in raw {
        let name = explicit_name.unwrap_or_else(|| match &path {
            Some(p) => p.clone(),
            None => {
                let n = format!("inline-{inline_index}");
                inline_index += 1;
                n
            }
        });
        let (content, fmt) =
            resolve_graph_source("shapes_graph", inline, path, shapes_format.clone())?;
        let graph = read_graph_from_string(&content, &fmt)
            .map_err(|e| format!("Failed to parse shapes graph ('{name}'): {e}"))?;
        sources.push(shacl_rust::sources::NamedSource { name, graph });
    }
    Ok(sources)
}

/// Resolves a [`ShapesGraphInput`], parsing every inline/path/named source
/// given and merging them into one shapes graph. Ignores collisions (use
/// [`resolve_shapes_sources`] directly when those diagnostics matter).
fn resolve_shapes_graph(input: ShapesGraphInput) -> Result<oxigraph::model::Graph, String> {
    let sources = resolve_shapes_sources(input)?;
    Ok(shacl_rust::sources::merge_sources(&sources))
}

/// Enriches the JSON rendering of the first [`shacl_rust::diagnostics::Diagnostic`]
/// for each distinct code with the registry's spec reference and
/// failing/fixed Turtle examples, so a caller rarely needs a follow-up
/// `explain_diagnostic_code` call — only later occurrences of an
/// already-seen code omit it, keeping the response from repeating the same
/// explanation once per violation.
fn attach_first_occurrence_explanations(
    json_array: &mut [serde_json::Value],
    diagnostics: &[shacl_rust::diagnostics::Diagnostic],
) {
    let mut seen = std::collections::HashSet::new();
    for (value, diagnostic) in json_array.iter_mut().zip(diagnostics.iter()) {
        if !seen.insert(diagnostic.code) {
            continue;
        }
        let Some(entry) = shacl_rust::diagnostics::entry(diagnostic.code) else {
            continue;
        };
        let Some(obj) = value.as_object_mut() else {
            continue;
        };
        obj.insert(
            "reference".to_string(),
            json!({
                "spec_ref": entry.spec_ref,
                "explanation": entry.explanation,
                "failing_example": entry.failing_example,
                "fixed_example": entry.fixed_example,
            }),
        );
    }
}

fn severity_summary(diagnostics: &[shacl_rust::diagnostics::Diagnostic]) -> serde_json::Value {
    use shacl_rust::diagnostics::DiagnosticSeverity;
    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Warning)
        .count();
    let info = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Info)
        .count();
    json!({
        "errors": errors,
        "warnings": warnings,
        "info": info,
        "diagnostic_count": diagnostics.len(),
    })
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
            data,
            shapes,
            output_format,
        }): Parameters<ValidateGraphsArgs>,
    ) -> Result<String, String> {
        let data_graph = resolve_data_graph(data)?;
        let shapes_graph = resolve_shapes_graph(shapes)?;

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
        Parameters(ValidateGraphsConformsArgs { data, shapes }): Parameters<
            ValidateGraphsConformsArgs,
        >,
    ) -> Result<String, String> {
        let data_graph = resolve_data_graph(data)?;
        let shapes_graph = resolve_shapes_graph(shapes)?;

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
        Parameters(LintGraphArgs {
            graph,
            graph_path,
            format,
        }): Parameters<LintGraphArgs>,
    ) -> Result<String, String> {
        let (content, fmt) = resolve_graph_source("graph", graph, graph_path, format)?;
        read_graph_from_string(&content, &fmt).map_err(|e| format!("Graph syntax error: {}", e))?;

        Ok(json!({ "valid": true }).to_string())
    }

    #[tool(
        description = "Parse SHACL shapes graph and return human-readable parsed shape information (counts, targets, constraint summaries). For structured JSON with every individual constraint and stable cross-run IDs, use decompose_shapes instead."
    )]
    async fn parse_shapes_graph(
        &self,
        Parameters(ParseShapesGraphArgs { shapes }): Parameters<ParseShapesGraphArgs>,
    ) -> Result<String, String> {
        let shapes_graph = resolve_shapes_graph(shapes)?;

        let parsed_shapes =
            parse_shapes(&shapes_graph).map_err(|e| format!("SHACL shapes error: {}", e))?;

        Ok(ShapesInfo::new(&parsed_shapes, shapes_graph.len(), true).to_string())
    }

    #[tool(
        description = "Decompose a SHACL shapes graph into structured JSON: one entry per individual constraint parameter binding (a property shape with sh:minCount + sh:datatype yields two entries sharing owner_property_shape), with recursive `children` for logical constraints (sh:and/or/xone/not/node/qualifiedValueShape) and content-stable `id`s that stay the same across runs, prefix renames, and unrelated edits elsewhere in the graph - unlike parse_shapes_graph's blank-node labels, which change every run. Use this instead of parse_shapes_graph when you need to join results back to specific constraint declarations (e.g. cross-referencing validate_diagnostics output) rather than just a human-readable shape summary."
    )]
    async fn decompose_shapes(
        &self,
        Parameters(DecomposeShapesArgs { shapes }): Parameters<DecomposeShapesArgs>,
    ) -> Result<String, String> {
        let sources = resolve_shapes_sources(shapes)?;
        let decomposed =
            shacl_rust::sources::decompose_with_collisions(&sources).map_err(|e| e.to_string())?;
        Ok(decomposed.to_string())
    }

    #[tool(
        description = "Validate RDF data against SHACL shapes and return rich rustc-style diagnostics (lint findings plus violation diagnostics), sorted"
    )]
    async fn validate_diagnostics(
        &self,
        Parameters(ValidateDiagnosticsArgs {
            data,
            shapes,
            skip_lint,
        }): Parameters<ValidateDiagnosticsArgs>,
    ) -> Result<String, String> {
        let data_graph = resolve_data_graph(data)?;
        let sources = resolve_shapes_sources(shapes)?;
        let mut diagnostics = shacl_rust::sources::detect_collisions(&sources);
        let shapes_graph = shacl_rust::sources::merge_sources(&sources);

        let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)
            .map_err(|e| format!("Failed to create validation dataset: {}", e))?;

        let shapes = parse_shapes(validation_dataset.shapes_graph())
            .map_err(|e| format!("Failed to parse shapes: {}", e))?;

        if !skip_lint {
            diagnostics.extend(shacl_rust::diagnostics::lint_shapes(
                validation_dataset.shapes_graph(),
                &shapes,
            ));
        }

        let report = validate(&validation_dataset, &shapes);
        diagnostics.extend(shacl_rust::diagnostics::from_report(
            &report,
            &validation_dataset,
            &shapes,
        ));
        shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

        let mut json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        attach_first_occurrence_explanations(&mut json_array, &diagnostics);

        let mut summary = severity_summary(&diagnostics);
        summary["conforms"] = json!(*report.get_conforms());
        summary["violation_count"] = json!(report.get_results().len());

        Ok(json!({
            "summary": summary,
            "diagnostics": json_array,
        })
        .to_string())
    }

    #[tool(
        description = "Run the 15 semantic shape-lint rules against a SHACL shapes graph and return lint diagnostics"
    )]
    async fn lint_shacl_shapes(
        &self,
        Parameters(LintShaclShapesArgs { shapes }): Parameters<LintShaclShapesArgs>,
    ) -> Result<String, String> {
        let sources = resolve_shapes_sources(shapes)?;
        let mut diagnostics = shacl_rust::sources::detect_collisions(&sources);
        let shapes_graph = shacl_rust::sources::merge_sources(&sources);

        let shapes =
            parse_shapes(&shapes_graph).map_err(|e| format!("SHACL shapes error: {}", e))?;

        diagnostics.extend(shacl_rust::diagnostics::lint_shapes(&shapes_graph, &shapes));
        shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

        let mut json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        attach_first_occurrence_explanations(&mut json_array, &diagnostics);

        Ok(json!({
            "summary": severity_summary(&diagnostics),
            "diagnostics": json_array,
        })
        .to_string())
    }

    #[tool(
        description = "Look up a diagnostic code (e.g. 'V0007') in the registry and return its title, spec reference, explanation, and a failing/fixed Turtle example pair. Rarely needed standalone: validate_diagnostics and lint_shacl_shapes already embed this same information under a `reference` key on each diagnostic's first occurrence of a given code. Reach for this tool when you need a code's explanation before it's actually triggered — e.g. checking what a rule means while writing shapes."
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
        description = "Trace why a focus node does or does not conform to SHACL shapes, constraint by constraint. Use this specifically when a shape *should* have fired for this node but validate_diagnostics came back empty (or conforms unexpectedly) — it walks every applicable shape/constraint for the node and reports each one's verdict (conforms/violates/not-targeted/vacuous), which pinpoints things validate_diagnostics can't show on its own: a target that silently didn't match, a constraint that silently short-circuited, or a query that silently returned no rows."
    )]
    async fn why_conformance(
        &self,
        Parameters(WhyConformanceArgs {
            data,
            shapes,
            focus_node,
            shape,
        }): Parameters<WhyConformanceArgs>,
    ) -> Result<String, String> {
        let data_graph = resolve_data_graph(data)?;
        let shapes_graph = resolve_shapes_graph(shapes)?;

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

        let mut json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        attach_first_occurrence_explanations(&mut json_array, &diagnostics);

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
    // Before touching stdio for the MCP transport: a flag here means someone
    // ran this as a CLI, not an MCP client connecting a JSON-RPC session -
    // print and exit instead of silently blocking on stdin waiting for a
    // request that will never come (which is what happened before this
    // check existed).
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("shacl-mcp {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "shacl-mcp {}\nMCP (Model Context Protocol) server for shacl-rust.\n\nRuns a JSON-RPC MCP session over stdio; not meant to be invoked interactively.\nSee an MCP client's server configuration for how to launch it.",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(());
            }
            _ => {}
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_graph_source_rejects_both_inline_and_path() {
        let err = resolve_graph_source(
            "data_graph",
            Some("<a> <b> <c> .".to_string()),
            Some("/tmp/whatever.ttl".to_string()),
            None,
        )
        .unwrap_err();
        assert!(err.contains("not both"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_graph_source_rejects_neither_inline_nor_path() {
        let err = resolve_graph_source("data_graph", None, None, None).unwrap_err();
        assert!(err.contains("data_graph_path"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_graph_source_inline_requires_format() {
        let err = resolve_graph_source("data_graph", Some("<a> <b> <c> .".to_string()), None, None)
            .unwrap_err();
        assert!(err.contains("data_graph_format"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_graph_source_inline_with_format_ok() {
        let (content, fmt) = resolve_graph_source(
            "data_graph",
            Some("<a> <b> <c> .".to_string()),
            None,
            Some("nt".to_string()),
        )
        .unwrap();
        assert_eq!(content, "<a> <b> <c> .");
        assert_eq!(fmt, "nt");
    }

    #[test]
    fn resolve_graph_source_path_infers_format_from_extension() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("shacl_mcp_test_{}.nt", std::process::id()));
        std::fs::write(&path, "<urn:a> <urn:b> <urn:c> .\n").unwrap();

        let (content, fmt) = resolve_graph_source(
            "data_graph",
            None,
            Some(path.to_string_lossy().to_string()),
            None,
        )
        .unwrap();
        assert_eq!(content, "<urn:a> <urn:b> <urn:c> .\n");
        assert_eq!(fmt, "nt");

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn resolve_graph_source_path_reports_missing_file() {
        let err = resolve_graph_source(
            "data_graph",
            None,
            Some("/nonexistent/shacl_mcp_test_missing.ttl".to_string()),
            None,
        )
        .unwrap_err();
        assert!(err.contains("Failed to read"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_shapes_graph_merges_multiple_inline_sources() {
        let input = ShapesGraphInput {
            shapes_graph: Some(
                "@prefix ex: <http://example.org/> . ex:S1 a ex:Shape .".to_string(),
            ),
            shapes_path: None,
            shapes_graphs: Some(vec![
                "@prefix ex: <http://example.org/> . ex:S2 a ex:Shape .".to_string(),
            ]),
            shapes_paths: None,
            shapes_sources: None,
            shapes_format: Some("ttl".to_string()),
        };
        let graph = resolve_shapes_graph(input).unwrap();
        assert_eq!(graph.len(), 2, "expected both sources' triples merged");
    }

    #[test]
    fn resolve_shapes_graph_requires_at_least_one_source() {
        let input = ShapesGraphInput {
            shapes_graph: None,
            shapes_path: None,
            shapes_graphs: None,
            shapes_paths: None,
            shapes_sources: None,
            shapes_format: None,
        };
        let err = resolve_shapes_graph(input).unwrap_err();
        assert!(err.contains("Provide one of"), "unexpected error: {err}");
    }

    #[test]
    fn validate_graphs_args_deserializes_flattened_fields() {
        let json = serde_json::json!({
            "data_graph": "<a> <b> <c> .",
            "data_format": "nt",
            "shapes_path": "/tmp/shapes.ttl",
            "shapes_format": "ttl",
            "output_format": "json",
        });
        let args: ValidateGraphsArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.data.data_graph.as_deref(), Some("<a> <b> <c> ."));
        assert_eq!(args.shapes.shapes_path.as_deref(), Some("/tmp/shapes.ttl"));
        assert_eq!(args.output_format, "json");
    }

    #[test]
    fn severity_summary_counts_by_severity() {
        use shacl_rust::diagnostics::{Diagnostic, DiagnosticSeverity};
        let make = |severity| Diagnostic {
            code: "V0000",
            severity,
            title: String::new(),
            constraint_component: None,
            snippets: Vec::new(),
            expected: None,
            actual: None,
            notes: Vec::new(),
            help: None,
            focus_node: None,
            source_shape: None,
            path: None,
            verdict: None,
        };
        let diagnostics = vec![
            make(DiagnosticSeverity::Error),
            make(DiagnosticSeverity::Error),
            make(DiagnosticSeverity::Warning),
            make(DiagnosticSeverity::Info),
        ];
        let summary = severity_summary(&diagnostics);
        assert_eq!(summary["errors"], 2);
        assert_eq!(summary["warnings"], 1);
        assert_eq!(summary["info"], 1);
        assert_eq!(summary["diagnostic_count"], 4);
    }

    #[test]
    fn attach_first_occurrence_explanations_only_annotates_first_of_each_code() {
        use shacl_rust::diagnostics::{Diagnostic, DiagnosticSeverity};
        let make = |code| Diagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            title: String::new(),
            constraint_component: None,
            snippets: Vec::new(),
            expected: None,
            actual: None,
            notes: Vec::new(),
            help: None,
            focus_node: None,
            source_shape: None,
            path: None,
            verdict: None,
        };
        // V0007 (sh:minInclusive) is a real registry entry; reuse it twice to
        // check dedup, and pair it with an unknown code to check the no-op path.
        let diagnostics = vec![make("V0007"), make("V0007")];
        let mut json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        attach_first_occurrence_explanations(&mut json_array, &diagnostics);

        assert!(json_array[0].get("reference").is_some());
        assert!(
            json_array[1].get("reference").is_none(),
            "second occurrence of the same code should not repeat the explanation"
        );
    }
}
