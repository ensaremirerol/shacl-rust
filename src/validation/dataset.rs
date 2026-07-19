use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

use oxigraph::{
    model::{Graph, GraphNameRef, NamedNodeRef, QuadRef},
    store::Store,
};

use crate::err::ShaclError;

pub const SHAPES_GRAPH_IRI: &str = "urn:shacl:shapes-graph";

#[derive(Clone)]
pub struct ValidationDataset {
    // Built lazily on first `store()` call: only SPARQL-based constraints read it,
    // and building it copies both graphs into the store. Shared across clones so
    // it is built at most once per dataset.
    store: Arc<OnceLock<Arc<Store>>>,
    data_graph: Graph,
    shapes_graph: Graph,
}

impl ValidationDataset {
    pub fn from_graphs(data_graph: Graph, shapes_graph: Graph) -> Result<Self, ShaclError> {
        Ok(Self {
            store: Arc::new(OnceLock::new()),
            data_graph,
            shapes_graph,
        })
    }

    pub fn store(&self) -> Result<Arc<Store>, ShaclError> {
        if let Some(store) = self.store.get() {
            return Ok(Arc::clone(store));
        }
        let built = Arc::new(Self::build_store(&self.data_graph, &self.shapes_graph)?);
        // Under contention another thread may have won the race; get_or_init
        // returns the stored value either way.
        Ok(Arc::clone(self.store.get_or_init(|| built)))
    }

    fn build_store(data_graph: &Graph, shapes_graph: &Graph) -> Result<Store, ShaclError> {
        let store = Store::new()
            .map_err(|e| ShaclError::Io(format!("Failed to create validation store: {}", e)))?;

        let shapes_graph_name = NamedNodeRef::new_unchecked(SHAPES_GRAPH_IRI);
        let quads = data_graph
            .iter()
            .map(|triple| {
                QuadRef::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphNameRef::DefaultGraph,
                )
            })
            .chain(shapes_graph.iter().map(|triple| {
                QuadRef::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphNameRef::NamedNode(shapes_graph_name),
                )
            }));

        store.extend(quads).map_err(|e| {
            ShaclError::Io(format!(
                "Failed to load graphs into validation store: {}",
                e
            ))
        })?;

        Ok(store)
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
