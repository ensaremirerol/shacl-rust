use std::io::BufReader;

use oxigraph::{io::JsonLdProfileSet, model::Triple};

use crate::err::ShaclError;

pub fn read_graph_from_string(
    graph_string: &str,
    file_format: &str,
) -> Result<oxigraph::model::Graph, ShaclError> {
    log::debug!("Reading graph from string, format: {}", file_format);
    let reader = BufReader::new(graph_string.as_bytes());
    read_graph_using_reader(reader, file_format)
}

pub fn read_graph(path: &str, file_format: &str) -> Result<oxigraph::model::Graph, ShaclError> {
    log::debug!("Reading graph from file: {}, format: {}", path, file_format);
    let file = std::fs::File::open(path)
        .map_err(|e| ShaclError::Io(format!("Failed to open file: {}", e)))?;
    let reader = std::io::BufReader::new(file);
    read_graph_using_reader(reader, file_format)
}

fn read_graph_using_reader<R: std::io::Read>(
    reader: BufReader<R>,
    file_format: &str,
) -> Result<oxigraph::model::Graph, ShaclError> {
    log::debug!("Reading graph from reader, format: {}", file_format);
    let mut graph = oxigraph::model::Graph::new();
    let parser = match file_format {
        "turtle" => oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::Turtle),
        "ttl" => oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::Turtle),
        "ntriples" => oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::NTriples),
        "nt" => oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::NTriples),
        "nquads" => oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::NQuads),
        "rdfxml" => oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::RdfXml),
        "jsonld" => oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::JsonLd {
            profile: JsonLdProfileSet::default(),
        }),
        _ => return Err(ShaclError::Io("Unsupported file format".into())),
    };
    let quads: Vec<oxigraph::model::Quad> = parser
        .with_base_iri("http://example.org")
        .map_err(|e| ShaclError::Io(format!("Failed to set base IRI: {}", e)))?
        .for_reader(reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ShaclError::Io(format!("Failed to parse the reader: {}", e)))?;
    for quad in quads {
        graph.insert(&Triple::from(quad));
    }
    Ok(graph)
}
