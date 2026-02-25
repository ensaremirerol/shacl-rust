use clap::{Parser, Subcommand};
use log::{debug, info};
use rayon::prelude::*;
use shacl_rust::{
    core::{shape::Shape, ShapesInfo},
    err::{path_to_str, ShaclError},
    parser, rdf, validate,
    validation::dataset::ValidationDataset,
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
    println!("{}", ShapesInfo::new(&shapes, graph.len(), detailed));

    Ok(())
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
    // If quiet is set, override log level to error
    if quiet {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();
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

    let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)?;

    // Parse shapes
    let shapes = parser::parse_shapes(validation_dataset.shapes_graph())?;
    info!("Parsed {} shapes", shapes.len());

    let report = validate(&validation_dataset, &shapes);

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
            let rdf_format = RdfFormat::from_extension(output_format).ok_or_else(|| {
                ShaclError::Parse(format!(
                    "Unsupported output format: '{}'. Supported: text, json, yaml, ttl, nt, nq, rdf, jsonld, trig",
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
