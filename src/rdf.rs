use std::io::BufReader;

use oxigraph::{
    io::{RdfFormat, RdfParser},
    model::Triple,
};

use crate::err::ShaclError;

fn normalize_rdf_format(file_format: &str) -> String {
    match file_format.trim().to_ascii_lowercase().as_str() {
        "turtle" => "ttl".to_string(),
        "ntriples" | "n-triples" => "nt".to_string(),
        "nquads" | "n-quads" => "nq".to_string(),
        "xml" | "rdfxml" | "rdf-xml" => "rdf".to_string(),
        "json-ld" => "jsonld".to_string(),
        other => other.to_string(),
    }
}

pub fn read_graph_from_string(
    graph_string: &str,
    file_format: &str,
) -> Result<oxigraph::model::Graph, ShaclError> {
    log::debug!("Reading graph from string, format: {}", file_format);
    let reader = BufReader::new(graph_string.as_bytes());
    read_graph_using_reader_with_base(reader, file_format, "http://example.org")
}

fn read_graph_using_reader_with_base<R: std::io::Read>(
    reader: BufReader<R>,
    file_format: &str,
    base_iri: &str,
) -> Result<oxigraph::model::Graph, ShaclError> {
    let normalized_format = normalize_rdf_format(file_format);

    let mut graph = oxigraph::model::Graph::new();

    let format = RdfFormat::from_extension(&normalized_format).ok_or_else(|| {
        ShaclError::Parse(format!(
            "Unsupported file extension: '{}'. Supported: ttl (turtle), nt (n-triples), nq (n-quads), rdf (rdfxml/xml), jsonld (json-ld), trig",
            file_format
        ))
    })?;

    let parser = RdfParser::from_format(format);
    let quads = parser
        .with_base_iri(base_iri)
        .map_err(|e| ShaclError::Parse(format!("Invalid base IRI '{}': {}", base_iri, e)))?
        .for_reader(reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ShaclError::Parse(format!("Failed to parse RDF data: {}", e)))?;

    graph.extend(quads.into_iter().map(Triple::from));

    Ok(graph)
}

pub fn serialize_graph_to_string(
    graph: &oxigraph::model::Graph,
    rdf_format: RdfFormat,
) -> Result<String, ShaclError> {
    let mut output = Vec::new();
    let mut serializer = oxigraph::io::RdfSerializer::from_format(rdf_format)
        .with_prefix("sh", "http://www.w3.org/ns/shacl#")
        .unwrap()
        .for_writer(&mut output);

    for triple in graph.iter() {
        serializer
            .serialize_triple(triple)
            .map_err(|e| ShaclError::Io(format!("Failed to serialize triple {}: {}", triple, e)))?;
    }

    serializer
        .finish()
        .map_err(|e| ShaclError::Io(format!("Failed to finalize serialized graph: {}", e)))?;

    String::from_utf8(output)
        .map_err(|e| ShaclError::Io(format!("Failed to serialize graph: {}", e)))
}
