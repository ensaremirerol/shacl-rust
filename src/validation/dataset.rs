use std::{ops::Deref, sync::Arc};

use oxigraph::{
    model::{Graph, GraphNameRef, NamedNodeRef, QuadRef},
    store::Store,
};

use crate::err::ShaclError;

pub const SHAPES_GRAPH_IRI: &str = "urn:shacl:shapes-graph";

#[derive(Clone)]
pub struct ValidationDataset {
    store: Arc<Store>,
    data_graph: Graph,
    shapes_graph: Graph,
}

impl ValidationDataset {
    pub fn from_graphs(data_graph: Graph, shapes_graph: Graph) -> Result<Self, ShaclError> {
        let store = Store::new()
            .map_err(|e| ShaclError::Io(format!("Failed to create validation store: {}", e)))?;

        for triple in data_graph.iter() {
            store
                .insert(QuadRef::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphNameRef::DefaultGraph,
                ))
                .map_err(|e| {
                    ShaclError::Io(format!(
                        "Failed to load data graph into validation store: {}",
                        e
                    ))
                })?;
        }

        let shapes_graph_name = NamedNodeRef::new_unchecked(SHAPES_GRAPH_IRI);
        for triple in shapes_graph.iter() {
            store
                .insert(QuadRef::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphNameRef::NamedNode(shapes_graph_name),
                ))
                .map_err(|e| {
                    ShaclError::Io(format!(
                        "Failed to load shapes graph into validation store: {}",
                        e
                    ))
                })?;
        }

        Ok(Self {
            store: Arc::new(store),
            data_graph,
            shapes_graph,
        })
    }

    pub fn store(&self) -> Arc<Store> {
        Arc::clone(&self.store)
    }

    pub fn data_graph(&self) -> &Graph {
        &self.data_graph
    }

    pub fn shapes_graph(&self) -> &Graph {
        &self.shapes_graph
    }
}

impl Deref for ValidationDataset {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.data_graph
    }
}
