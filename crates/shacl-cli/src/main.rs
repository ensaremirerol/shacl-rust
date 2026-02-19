use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};
use rayon::prelude::*;
use shacl_rust::{
    core::shape::Shape,
    err::{path_to_str, ShaclError},
    parser, rdf,
    validation::build_target_cache,
    validation::dataset::{register_store_for_graph, ValidationDataset},
    validation::{report::ValidationReport, report::ValidationResult},
};
use std::fmt::{Display, Formatter};
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

        /// Output format as file extension (ttl, nt, nq, rdf, jsonld, trig, json, yaml)
        /// If omitted or 'text', prints human-readable format. Otherwise exports as RDF graph.
        #[arg(long, default_value = "text")]
        output_format: String,

        /// Disable progress output
        #[arg(long, visible_alias = "quite")]
        quiet: bool,
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
}

fn main() -> Result<(), ShaclError> {
    let cli = Cli::parse();

    // Initialize logger based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
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
            quiet,
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
                quiet,
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
    println!(
        "{}",
        ShapesInfo {
            shapes: &shapes,
            graph_len: graph.len(),
            detailed,
        }
    );

    Ok(())
}

struct ShapesInfo<'a> {
    shapes: &'a [Shape<'a>],
    graph_len: usize,
    detailed: bool,
}

impl Display for ShapesInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n{}", "=".repeat(80))?;
        writeln!(f, "SHACL Shapes Information")?;
        writeln!(f, "{}", "=".repeat(80))?;
        writeln!(f, "Total shapes: {}", self.shapes.len())?;
        writeln!(f, "Total triples in shapes graph: {}", self.graph_len)?;

        let active_shapes = self.shapes.iter().filter(|s| !s.deactivated).count();
        let deactivated_shapes = self.shapes.len() - active_shapes;

        let total_targets: usize = self.shapes.iter().map(|s| s.targets.len()).sum();
        let total_constraints: usize = self.shapes.iter().map(|s| s.constraints.len()).sum();

        writeln!(f, "\nShape Status:")?;
        writeln!(f, "  Active: {}", active_shapes)?;
        writeln!(f, "  Deactivated: {}", deactivated_shapes)?;

        writeln!(f, "\nConstraints:")?;
        writeln!(f, "  Total targets: {}", total_targets)?;
        writeln!(f, "  Total constraints: {}", total_constraints)?;

        if self.detailed {
            writeln!(f, "\n{}", "-".repeat(80))?;
            writeln!(f, "Detailed Shape Information:")?;
            writeln!(f, "{}", "-".repeat(80))?;

            for (idx, shape) in self.shapes.iter().enumerate() {
                writeln!(f, "\nShape #{}: {}", idx + 1, shape.node)?;
                writeln!(
                    f,
                    "  Status: {}",
                    if shape.deactivated {
                        "DEACTIVATED"
                    } else {
                        "ACTIVE"
                    }
                )?;
                writeln!(f, "  Severity: {}", shape.severity)?;
                writeln!(f, "  Targets: {}", shape.targets.len())?;

                for target in &shape.targets {
                    writeln!(f, "    - {}", target)?;
                }

                writeln!(f, "  Constraints: {}", shape.constraints.len())?;

                for constraint in &shape.constraints {
                    writeln!(f, "    - {}", constraint)?;
                }

                if let Some(closed) = &shape.closed {
                    writeln!(f, "  Closed: {}", closed)?;
                }

                if !shape.message.is_empty() {
                    writeln!(f, "  Messages: {}", shape.message.len())?;
                    for msg in &shape.message {
                        writeln!(f, "    - {}", msg)?;
                    }
                }
            }
        }

        writeln!(f, "\n{}", "=".repeat(80))
    }
}

fn validate_command(
    shapes_file: PathBuf,
    data_files: Vec<PathBuf>,
    data_format: Option<String>,
    shapes_format: Option<String>,
    output: Option<PathBuf>,
    output_format: &str,
    quiet: bool,
) -> Result<(), ShaclError> {
    let progress_style =
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .map_err(|e| {
                ShaclError::Parse(format!("Failed to configure progress bar style: {}", e))
            })?
            .progress_chars("##-");

    let data_files_bar = if quiet {
        None
    } else {
        let bar = ProgressBar::new(data_files.len() as u64);
        bar.set_style(progress_style.clone());
        bar.set_message("Loading data files");
        Some(bar)
    };

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
                if let Some(ref bar) = data_files_bar {
                    bar.inc(1);
                }
                Ok((data_file, graph))
            })
            .collect();

    if let Some(bar) = data_files_bar {
        bar.finish_with_message("Loaded data files");
    }

    let mut data_graph = oxigraph::model::Graph::new();
    for data_graph_result in data_graphs_results {
        let (data_file, graph) = data_graph_result?;
        let before_len = data_graph.len();
        data_graph.extend(graph.iter().map(oxigraph::model::Triple::from));
        info!(
            "Merged data graph {} ({} triples, total now {})",
            data_file.display(),
            graph.len(),
            data_graph.len()
        );
        debug!(
            "Data merge added {} unique triples",
            data_graph.len().saturating_sub(before_len)
        );
    }

    debug!(
        "Reading shapes graph from {} with format {}",
        shapes_file.display(),
        shapes_format.as_deref().unwrap_or("auto")
    );

    // Load shapes graph
    let shapes_graph = read_graph_from_file(&shapes_file, shapes_format.as_deref())?;
    info!("Shapes graph loaded with {} triples", shapes_graph.len());

    // Parse shapes
    let shapes = parser::parse_shapes(&shapes_graph)?;
    info!("Parsed {} shapes", shapes.len());

    let validation_dataset = ValidationDataset::from_graphs(data_graph, &shapes_graph)?;
    register_store_for_graph(validation_dataset.data_graph(), validation_dataset.store());

    let validation_bar = if quiet {
        None
    } else {
        let bar = ProgressBar::new(shapes.len() as u64);
        bar.set_style(progress_style);
        bar.set_message("Validating shapes");
        Some(bar)
    };

    // Run validation for all shapes
    let mut combined_report = ValidationReport::new();
    let target_cache = build_target_cache(validation_dataset.data_graph(), &shapes);

    for shape in &shapes {
        let report =
            shape.validate_with_target_cache(validation_dataset.data_graph(), &target_cache);

        // Merge reports
        if !report.conforms {
            combined_report.conforms = false;
        }
        combined_report.results.extend(report.results);

        if let Some(ref bar) = validation_bar {
            bar.inc(1);
        }
    }

    if let Some(bar) = validation_bar {
        bar.finish_with_message("Validation completed");
    }

    // Determine output format and generate report
    let output_text = match output_format {
        "text" => {
            // Human-readable text format
            combined_report.to_string()
        }
        "json" => {
            // JSON format
            format_validation_report_json(&combined_report)?
        }
        _ => {
            // Try to parse as RDF format (ttl, nt, nq, rdf, jsonld, trig)
            use oxigraph::io::RdfFormat;
            let rdf_format = RdfFormat::from_extension(output_format).ok_or_else(|| {
                ShaclError::Parse(format!(
                    "Unsupported output format: '{}'. Supported: text, json, yaml, ttl, nt, nq, rdf, jsonld, trig",
                    output_format
                ))
            })?;

            // Convert validation report to RDF graph
            let report_graph = combined_report.to_graph();

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
    if !combined_report.conforms {
        std::process::exit(1);
    }

    Ok(())
}

fn read_graph_from_file(
    path: &Path,
    format: Option<&str>,
) -> Result<oxigraph::model::Graph, ShaclError> {
    let content = std::fs::read_to_string(path_to_str(path)?).map_err(|e| {
        ShaclError::Io(format!(
            "Failed to read graph file '{}': {}",
            path.display(),
            e
        ))
    })?;

    let effective_format = format.or_else(|| path.extension().and_then(|ext| ext.to_str()));
    let effective_format = effective_format.ok_or_else(|| {
        ShaclError::Parse(format!(
            "Could not infer RDF format for '{}'. Please provide --format.",
            path.display()
        ))
    })?;
    rdf::read_graph_from_string(&content, effective_format)
}

fn format_validation_report_json(report: &ValidationReport) -> Result<String, ShaclError> {
    use serde_json::json;

    let results_json: Vec<_> = report
        .results
        .iter()
        .map(validation_result_to_json)
        .collect();

    let output = json!({
        "conforms": report.conforms,
        "results": results_json,
    });

    serde_json::to_string_pretty(&output)
        .map_err(|e| ShaclError::Parse(format!("Failed to serialize to JSON: {}", e)))
}

fn validation_result_to_json(result: &ValidationResult) -> serde_json::Value {
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
