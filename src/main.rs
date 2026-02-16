pub mod constraints;
mod err;
mod io;
mod parser;
pub mod path;
pub mod vocab;

fn main() {
    let graph = io::read_graph("tests/resources/core/property/datatype-001.ttl", "turtle")
        .expect("Failed to read graph");
    println!("Graph has {} triples", graph.len());
}
