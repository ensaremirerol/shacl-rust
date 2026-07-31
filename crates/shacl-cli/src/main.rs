use clap::{Parser, Subcommand};
use log::{debug, info};
use rayon::prelude::*;
use shacl_rust::{
    core::{shape::Shape, ShapesInfo},
    diagnostics::{Diagnostic, DiagnosticSeverity},
    err::{path_to_str, ShaclError},
    parser, rdf, validate,
    validation::dataset::ValidationDataset,
};
use std::fmt::{Display, Formatter};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// SHACL (Shapes Constraint Language) validator and toolkit
#[derive(Parser)]
#[command(name = "shacl-validator")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the verbosity level (can be used multiple times: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and display SHACL shapes from a shapes graph
    Parse {
        /// Path to the SHACL shapes file
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,

        /// RDF format of the shapes file (ttl, nt, nq, rdf, jsonld, trig)
        /// If not specified, will be auto-detected from file extension
        #[arg(short, long)]
        format: Option<String>,

        /// Output format for displaying shapes (pretty, json, compact)
        #[arg(short, long, default_value = "pretty")]
        output: String,
    },

    /// Validate RDF data against SHACL shapes
    Validate {
        /// Path to the SHACL shapes file
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,

        /// Data files to validate (one or more)
        #[arg(value_name = "DATA_FILE", required = true)]
        data_files: Vec<PathBuf>,

        /// RDF format of the data file (auto-detected from extension if not specified)
        /// Supported: ttl, nt, nq, rdf, jsonld, trig
        #[arg(short = 'd', long)]
        data_format: Option<String>,

        /// RDF format of the shapes file (auto-detected from extension if not specified)
        /// Supported: ttl, nt, nq, rdf, jsonld, trig
        #[arg(short = 's', long)]
        shapes_format: Option<String>,

        /// Output file for validation report (if not specified, prints to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format as file extension (ttl/turtle, nt, nq, rdf, jsonld, trig, json, yaml)
        /// If omitted or 'text', prints human-readable format. Otherwise exports as RDF graph.
        #[arg(long, default_value = "text")]
        output_format: String,

        /// Disable progress output
        #[arg(long, visible_alias = "quite")]
        quiet: bool,

        /// EXPERIMENTAL: load the data graph into an interned parallel index
        /// instead of an oxigraph Graph (faster and leaner on large graphs)
        #[arg(long)]
        experimental_index: bool,

        /// Diagnostics output on stderr: text, json (NDJSON), or none
        #[arg(long, default_value = "text")]
        diagnostics: String,

        /// Skip shape lints during validation
        #[arg(long)]
        skip_lint: bool,

        /// Exit with code 2 when shape lints report warnings or errors
        #[arg(long)]
        deny_warnings: bool,
    },

    /// Show information about SHACL shapes
    Info {
        /// Path to the SHACL shapes file
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,

        /// RDF format of the shapes file (auto-detected from extension if not specified)
        /// Supported: ttl, nt, nq, rdf, jsonld, trig
        #[arg(short, long)]
        format: Option<String>,

        /// Show detailed statistics
        #[arg(short, long)]
        detailed: bool,
    },

    /// Lint a shapes graph without validating data
    Lint {
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,
        #[arg(short, long)]
        format: Option<String>,
        #[arg(long, default_value = "text")]
        diagnostics: String,
    },

    /// Explain a diagnostic code (e.g. V0007, L0002)
    Explain {
        #[arg(value_name = "CODE")]
        code: String,
    },

    /// Decompose a shapes graph into structured JSON: one entry per
    /// individual constraint parameter binding, with content-stable IDs
    /// independent of blank-node labels, prefixes, or unrelated edits
    /// elsewhere in the graph.
    Decompose {
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,
        #[arg(short, long)]
        format: Option<String>,
        /// Pretty-print the JSON output (default: compact, one line)
        #[arg(long)]
        pretty: bool,
    },

    /// Explain why a focus node does or does not fail shapes
    Why {
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,
        #[arg(value_name = "DATA_FILE")]
        data_file: PathBuf,
        /// Focus node IRI (angle brackets optional)
        #[arg(long)]
        focus: String,
        /// Restrict to one shape IRI
        #[arg(long)]
        shape: Option<String>,
        #[arg(long, default_value = "text")]
        diagnostics: String,
    },
}

fn main() -> Result<(), ShaclError> {
    let cli = Cli::parse();

    // Initialize logger based on verbosity (quiet, on the validate subcommand, forces "error")
    let quiet = matches!(cli.command, Commands::Validate { quiet: true, .. });
    let log_level = if quiet {
        "error"
    } else {
        match cli.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    debug!("Starting SHACL validator");

    match cli.command {
        Commands::Parse {
            shapes_file,
            format,
            output,
        } => {
            info!("Parsing shapes from: {}", shapes_file.display());
            parse_shapes_command(shapes_file, format, &output)
        }
        Commands::Validate {
            shapes_file,
            data_files,
            data_format,
            shapes_format,
            output,
            output_format,
            quiet: _,
            experimental_index,
            diagnostics,
            skip_lint,
            deny_warnings,
        } => {
            info!("Validating {} data file(s)", data_files.len());
            info!("Using shapes: {}", shapes_file.display());
            validate_command(
                shapes_file,
                data_files,
                data_format,
                shapes_format,
                output,
                &output_format,
                experimental_index,
                &diagnostics,
                skip_lint,
                deny_warnings,
            )
        }
        Commands::Info {
            shapes_file,
            format,
            detailed,
        } => {
            info!("Showing info for shapes: {}", shapes_file.display());
            info_command(shapes_file, format, detailed)
        }
        Commands::Lint {
            shapes_file,
            format,
            diagnostics,
        } => {
            info!("Linting shapes: {}", shapes_file.display());
            lint_command(shapes_file, format, &diagnostics)
        }
        Commands::Explain { code } => explain_command(&code),
        Commands::Decompose {
            shapes_file,
            format,
            pretty,
        } => decompose_command(shapes_file, format, pretty),
        Commands::Why {
            shapes_file,
            data_file,
            focus,
            shape,
            diagnostics,
        } => why_command(
            shapes_file,
            data_file,
            &focus,
            shape.as_deref(),
            &diagnostics,
        ),
    }
}

fn parse_shapes_command(
    shapes_file: PathBuf,
    format: Option<String>,
    output: &str,
) -> Result<(), ShaclError> {
    debug!(
        "Reading shapes graph from {} with format {}",
        shapes_file.display(),
        format.as_deref().unwrap_or("auto")
    );

    let graph = read_graph_from_file(&shapes_file, format.as_deref())?;

    info!("Graph loaded with {} triples", graph.len());

    let shapes = parser::parse_shapes(&graph)?;
    info!("Parsed {} shapes", shapes.len());

    match output {
        "pretty" => println!("{}", ShapesPretty(&shapes)),
        "json" => print_shapes_json(&shapes)?,
        "compact" => println!("{}", ShapesCompact(&shapes)),
        _ => {
            return Err(ShaclError::Parse(format!(
                "Unknown output format: {}. Use 'pretty', 'json', or 'compact'",
                output
            )))
        }
    }

    Ok(())
}

struct ShapesPretty<'a>(&'a [Shape<'a>]);

impl Display for ShapesPretty<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n{}", "=".repeat(80))?;
        writeln!(f, "Parsed {} SHACL Shape(s)", self.0.len())?;
        writeln!(f, "{}\n", "=".repeat(80))?;

        for (idx, shape) in self.0.iter().enumerate() {
            writeln!(f, "Shape #{}:", idx + 1)?;
            writeln!(f, "{}", shape)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

struct ShapesCompact<'a>(&'a [Shape<'a>]);

impl Display for ShapesCompact<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Parsed {} shape(s):", self.0.len())?;
        for (idx, shape) in self.0.iter().enumerate() {
            writeln!(
                f,
                "  {}. {} - {} target(s), {} constraint(s)",
                idx + 1,
                shape.node,
                shape.targets.len(),
                shape.constraints.len()
            )?;
        }
        Ok(())
    }
}

fn print_shapes_json(shapes: &[Shape<'_>]) -> Result<(), ShaclError> {
    use serde_json::json;

    let shapes_json: Vec<_> = shapes
        .iter()
        .map(|shape| {
            json!({
                "node": shape.node.to_string(),
                "name": shape.name,
                "targets": shape.targets.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
                "deactivated": shape.deactivated,
                "severity": shape.severity.to_string(),
                "messages": shape.message.iter().collect::<Vec<_>>(),
                "constraints": shape.constraints.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
                "closed": shape.closed.as_ref().map(|c| c.to_string()),
            })
        })
        .collect();

    let output = json!({
        "shapes": shapes_json,
        "count": shapes.len(),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .map_err(|e| { ShaclError::Parse(format!("Failed to serialize to JSON: {}", e)) })?
    );

    Ok(())
}

fn info_command(
    shapes_file: PathBuf,
    format: Option<String>,
    detailed: bool,
) -> Result<(), ShaclError> {
    debug!(
        "Reading shapes graph from {} with format {}",
        shapes_file.display(),
        format.as_deref().unwrap_or("auto")
    );

    let graph = read_graph_from_file(&shapes_file, format.as_deref())?;
    info!("Graph loaded with {} triples", graph.len());

    let shapes = parser::parse_shapes(&graph)?;
    println!("{}", ShapesInfo::new(&shapes, graph.len(), detailed));

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_command(
    shapes_file: PathBuf,
    data_files: Vec<PathBuf>,
    data_format: Option<String>,
    shapes_format: Option<String>,
    output: Option<PathBuf>,
    output_format: &str,
    experimental_index: bool,
    diagnostics_mode: &str,
    skip_lint: bool,
    deny_warnings: bool,
) -> Result<(), ShaclError> {
    if experimental_index {
        return validate_command_indexed(
            shapes_file,
            data_files,
            data_format,
            shapes_format,
            output,
            output_format,
            diagnostics_mode,
            skip_lint,
            deny_warnings,
        );
    }
    let data_graphs_results: Vec<Result<(PathBuf, oxigraph::model::Graph), ShaclError>> =
        data_files
            .into_par_iter()
            .map(|data_file| {
                debug!(
                    "Reading data graph from {} with format {}",
                    data_file.display(),
                    data_format.as_deref().unwrap_or("auto")
                );
                let graph = read_graph_from_file(&data_file, data_format.as_deref())?;
                info!(
                    "Data graph {} loaded with {} triples",
                    data_file.display(),
                    graph.len()
                );
                Ok((data_file, graph))
            })
            .collect();

    // Merging rebuilds the graph's indexes, which dominates load time and
    // doubles peak memory on large graphs — reuse the parsed graph when there
    // is nothing to merge.
    let mut merged: Option<oxigraph::model::Graph> = None;
    for data_graph_result in data_graphs_results {
        let (data_file, graph) = data_graph_result?;
        let triples = graph.len();
        match &mut merged {
            None => merged = Some(graph),
            Some(data_graph) => {
                let before_len = data_graph.len();
                data_graph.extend(graph.iter());
                info!(
                    "Merged data graph {} ({} triples, total now {})",
                    data_file.display(),
                    triples,
                    data_graph.len()
                );
                debug!(
                    "Data merge added {} unique triples",
                    data_graph.len().saturating_sub(before_len)
                );
            }
        }
    }
    let data_graph = merged.unwrap_or_default();

    debug!(
        "Reading shapes graph from {} with format {}",
        shapes_file.display(),
        shapes_format.as_deref().unwrap_or("auto")
    );

    // Load shapes graph
    let shapes_graph = read_graph_from_file(&shapes_file, shapes_format.as_deref())?;
    info!("Shapes graph loaded with {} triples", shapes_graph.len());

    let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)?;

    // Parse shapes
    let shapes = parser::parse_shapes(validation_dataset.shapes_graph())?;
    info!("Parsed {} shapes", shapes.len());

    let report = validate(&validation_dataset, &shapes);

    emit_diagnostics_and_maybe_exit(
        &report,
        &validation_dataset,
        &shapes,
        diagnostics_mode,
        skip_lint,
        deny_warnings,
    );

    emit_report(&report, output, output_format)
}

/// Lints (unless skipped) + derives diagnostics from the report, renders them
/// to stderr per `diagnostics_mode`, and exits 2 if `deny_warnings` is set and
/// any lint diagnostic (code starting with 'L') was produced. This runs
/// before `emit_report`'s own conformance exit(1), so deny-warnings exit 2
/// wins over conformance exit 1.
fn emit_diagnostics_and_maybe_exit(
    report: &shacl_rust::ValidationReport<'_>,
    validation_dataset: &ValidationDataset,
    shapes: &[Shape<'_>],
    diagnostics_mode: &str,
    skip_lint: bool,
    deny_warnings: bool,
) {
    let mut all_diags = Vec::new();
    if !skip_lint {
        all_diags.extend(shacl_rust::diagnostics::lint_shapes(
            validation_dataset.shapes_graph(),
            shapes,
        ));
    }
    all_diags.extend(shacl_rust::diagnostics::from_report(
        report,
        validation_dataset,
        shapes,
    ));
    shacl_rust::diagnostics::sort_diagnostics(&mut all_diags);

    emit_diagnostics(&all_diags, diagnostics_mode);

    let lint_warnings = all_diags.iter().any(|d| d.code.starts_with('L'));
    if deny_warnings && lint_warnings {
        std::process::exit(2);
    }
}

/// Renders `diags` to stderr per `mode` ("text", "json"/NDJSON, or "none").
fn emit_diagnostics(diags: &[Diagnostic], mode: &str) {
    match mode {
        "json" => eprint!("{}", shacl_rust::diagnostics::render_ndjson(diags)),
        "none" => {}
        _ => {
            let color = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
            eprint!("{}", shacl_rust::diagnostics::render_text(diags, color));
        }
    }
}

fn emit_report(
    report: &shacl_rust::ValidationReport<'_>,
    output: Option<PathBuf>,
    output_format: &str,
) -> Result<(), ShaclError> {
    // Determine output format and generate report
    let output_text = match output_format {
        "text" => {
            // Human-readable text format
            report.to_string()
        }
        "json" => {
            // JSON format
            report.as_json().to_string()
        }
        _ => {
            // Try to parse as RDF format (ttl, nt, nq, rdf, jsonld, trig)
            use oxigraph::io::RdfFormat;
            // "turtle" is the conventional RDF-ecosystem name for the "ttl" extension.
            let normalized_format = if output_format.eq_ignore_ascii_case("turtle") {
                "ttl"
            } else {
                output_format
            };
            let rdf_format = RdfFormat::from_extension(normalized_format).ok_or_else(|| {
                ShaclError::Parse(format!(
                    "Unsupported output format: '{}'. Supported: text, json, yaml, ttl/turtle, nt, nq, rdf, jsonld, trig",
                    output_format
                ))
            })?;

            // Convert validation report to RDF graph
            let report_graph = report.to_graph();

            // Serialize to string
            rdf::serialize_graph_to_string(&report_graph, rdf_format)?
        }
    };

    // Write output
    if let Some(output_path) = output {
        debug!("Writing report to {}", output_path.display());
        std::fs::write(&output_path, &output_text)
            .map_err(|e| ShaclError::Io(format!("Failed to write output file: {}", e)))?;
        info!("Report written to {}", output_path.display());
    } else {
        // Print to stdout
        println!("{}", output_text);
    }

    // Exit with error code if validation failed
    if !*report.get_conforms() {
        std::process::exit(1);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_command_indexed(
    shapes_file: PathBuf,
    data_files: Vec<PathBuf>,
    data_format: Option<String>,
    shapes_format: Option<String>,
    output: Option<PathBuf>,
    output_format: &str,
    diagnostics_mode: &str,
    skip_lint: bool,
    deny_warnings: bool,
) -> Result<(), ShaclError> {
    // Open every data file as a streaming parser; the serialized text is
    // never held in memory.
    let mut parsers = Vec::new();
    for data_file in data_files {
        let effective_format = data_format
            .as_deref()
            .or_else(|| data_file.extension().and_then(|ext| ext.to_str()))
            .ok_or_else(|| {
                ShaclError::Parse(format!(
                    "Could not infer RDF format for '{}'. Please provide --data-format.",
                    data_file.display()
                ))
            })?
            .to_string();
        let file = std::fs::File::open(path_to_str(&data_file)?).map_err(|e| {
            ShaclError::Io(format!(
                "Failed to read graph file '{}': {}",
                data_file.display(),
                e
            ))
        })?;
        parsers.push(rdf::parse_triples_from_reader(file, &effective_format)?);
    }

    let shapes_graph = read_graph_from_file(&shapes_file, shapes_format.as_deref())?;
    info!("Shapes graph loaded with {} triples", shapes_graph.len());

    // Stream every file straight into the index; the first parse error wins.
    let parse_error: std::cell::RefCell<Option<ShaclError>> = std::cell::RefCell::new(None);
    let triples = parsers
        .into_iter()
        .flatten()
        .filter_map(|result| match result {
            Ok(triple) => Some(triple),
            Err(e) => {
                parse_error.borrow_mut().get_or_insert(e);
                None
            }
        });
    let validation_dataset =
        ValidationDataset::from_triples_with_experimental_index(triples, shapes_graph)?;
    if let Some(e) = parse_error.into_inner() {
        return Err(e);
    }
    info!(
        "Data graph indexed with {} triples (experimental backend)",
        validation_dataset.data().len()
    );

    let shapes = parser::parse_shapes(validation_dataset.shapes_graph())?;
    info!("Parsed {} shapes", shapes.len());

    let report = validate(&validation_dataset, &shapes);

    emit_diagnostics_and_maybe_exit(
        &report,
        &validation_dataset,
        &shapes,
        diagnostics_mode,
        skip_lint,
        deny_warnings,
    );

    emit_report(&report, output, output_format)
}

fn lint_command(
    shapes_file: PathBuf,
    format: Option<String>,
    diagnostics_mode: &str,
) -> Result<(), ShaclError> {
    let graph = read_graph_from_file(&shapes_file, format.as_deref())?;
    info!("Shapes graph loaded with {} triples", graph.len());

    let shapes = parser::parse_shapes(&graph)?;
    info!("Parsed {} shapes", shapes.len());

    let mut diags = shacl_rust::diagnostics::lint_shapes(&graph, &shapes);
    shacl_rust::diagnostics::sort_diagnostics(&mut diags);

    emit_diagnostics(&diags, diagnostics_mode);

    if diags
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error)
    {
        std::process::exit(1);
    }

    Ok(())
}

fn decompose_command(
    shapes_file: PathBuf,
    format: Option<String>,
    pretty: bool,
) -> Result<(), ShaclError> {
    let graph = read_graph_from_file(&shapes_file, format.as_deref())?;
    info!("Shapes graph loaded with {} triples", graph.len());

    let shapes = parser::parse_shapes(&graph)?;
    info!("Parsed {} shapes", shapes.len());

    let decomposed = shacl_rust::decompose_shapes(&shapes, None, graph.len());
    let rendered = if pretty {
        serde_json::to_string_pretty(&decomposed)
    } else {
        serde_json::to_string(&decomposed)
    }
    .map_err(|e| ShaclError::Io(format!("Failed to serialize decomposition: {e}")))?;
    println!("{rendered}");

    Ok(())
}

fn explain_command(code: &str) -> Result<(), ShaclError> {
    match shacl_rust::diagnostics::entry(code) {
        Some(entry) => {
            println!("{}: {}", entry.code, entry.title);
            if let Some(component) = entry.component {
                println!("component: {}", component);
            }
            println!("spec: {}", entry.spec_ref);
            println!();
            println!("{}", entry.explanation);
            println!();
            println!("== failing example ==");
            println!("{}", entry.failing_example);
            println!();
            println!("== fix ==");
            println!("{}", entry.fixed_example);
            Ok(())
        }
        None => {
            eprintln!("Unknown diagnostic code: {}", code);
            std::process::exit(1);
        }
    }
}

/// Trims a single pair of optional surrounding angle brackets (`<...>`) from
/// an IRI argument, so both `--focus http://example.org/a` and
/// `--focus <http://example.org/a>` work.
fn trim_angle_brackets(s: &str) -> &str {
    s.trim().trim_start_matches('<').trim_end_matches('>')
}

/// Traces why `focus` does or does not conform to the shapes graph, printing
/// the resulting diagnostics to stdout - the trace *is* the product of this
/// subcommand, so it always exits 0 (it never fails on a violation).
fn why_command(
    shapes_file: PathBuf,
    data_file: PathBuf,
    focus: &str,
    shape: Option<&str>,
    diagnostics_mode: &str,
) -> Result<(), ShaclError> {
    let data_graph = read_graph_from_file(&data_file, None)?;
    let shapes_graph = read_graph_from_file(&shapes_file, None)?;
    let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)?;
    let shapes = parser::parse_shapes(validation_dataset.shapes_graph())?;

    let focus_iri = trim_angle_brackets(focus);
    let focus_node = oxigraph::model::NamedNode::new(focus_iri)
        .map_err(|e| ShaclError::Parse(format!("Invalid focus IRI '{}': {}", focus_iri, e)))?;
    let focus_term = validation_dataset
        .data()
        .canonical_term(oxigraph::model::TermRef::from(focus_node.as_ref()))
        .unwrap_or_else(|| oxigraph::model::TermRef::from(focus_node.as_ref()));

    let shape_node = match shape {
        Some(s) => {
            let shape_iri = trim_angle_brackets(s);
            Some(oxigraph::model::NamedNode::new(shape_iri).map_err(|e| {
                ShaclError::Parse(format!("Invalid shape IRI '{}': {}", shape_iri, e))
            })?)
        }
        None => None,
    };
    let shape_filter = shape_node
        .as_ref()
        .map(|n| oxigraph::model::NamedOrBlankNodeRef::from(n.as_ref()));

    let diags = shacl_rust::diagnostics::explain_conformance(
        &validation_dataset,
        &shapes,
        focus_term,
        shape_filter,
    );

    match diagnostics_mode {
        "json" => print!("{}", shacl_rust::diagnostics::render_ndjson(&diags)),
        _ => {
            let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
            print!("{}", shacl_rust::diagnostics::render_text(&diags, color));
        }
    }

    Ok(())
}

fn read_graph_from_file(
    path: &Path,
    format: Option<&str>,
) -> Result<oxigraph::model::Graph, ShaclError> {
    let effective_format = format.or_else(|| path.extension().and_then(|ext| ext.to_str()));
    let effective_format = effective_format.ok_or_else(|| {
        ShaclError::Parse(format!(
            "Could not infer RDF format for '{}'. Please provide --format.",
            path.display()
        ))
    })?;
    let file = std::fs::File::open(path_to_str(path)?).map_err(|e| {
        ShaclError::Io(format!(
            "Failed to read graph file '{}': {}",
            path.display(),
            e
        ))
    })?;
    rdf::read_graph_from_reader(file, effective_format)
}
