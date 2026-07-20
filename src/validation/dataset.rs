use std::sync::{Arc, OnceLock};

use oxigraph::{
    model::{Graph, GraphNameRef, NamedNodeRef, QuadRef, Triple},
    store::Store,
};

use crate::err::ShaclError;
use crate::indexed_graph::{DataView, IndexedGraph};

pub const SHAPES_GRAPH_IRI: &str = "urn:shacl:shapes-graph";

/// Storage backend for the data graph.
enum DataBackend {
    Graph(Graph),
    /// Experimental interned index backend; see [`crate::indexed_graph`].
    Indexed(IndexedGraph),
}

pub struct ValidationDataset {
    // Built lazily on first `store()` call: only SPARQL-based constraints read it,
    // and building it copies both graphs into the store. Shared across clones so
    // it is built at most once per dataset.
    store: Arc<OnceLock<Arc<Store>>>,
    data: DataBackend,
    shapes_graph: Graph,
}

impl ValidationDataset {
    pub fn from_graphs(data_graph: Graph, shapes_graph: Graph) -> Result<Self, ShaclError> {
        Ok(Self {
            store: Arc::new(OnceLock::new()),
            data: DataBackend::Graph(data_graph),
            shapes_graph,
        })
    }

    /// **Experimental**: like [`Self::from_graphs`], but stores the data graph
    /// in the interned [`IndexedGraph`] backend.
    pub fn from_graphs_with_experimental_index(
        data_graph: Graph,
        shapes_graph: Graph,
    ) -> Result<Self, ShaclError> {
        let index = IndexedGraph::from_graph(&data_graph);
        Ok(Self {
            store: Arc::new(OnceLock::new()),
            data: DataBackend::Indexed(index),
            shapes_graph,
        })
    }

    /// **Experimental**: builds the interned data-graph index directly from a
    /// triple stream, skipping the intermediate [`Graph`] entirely. This is
    /// the cheapest way to load large data graphs.
    pub fn from_triples_with_experimental_index(
        data_triples: impl IntoIterator<Item = Triple>,
        shapes_graph: Graph,
    ) -> Result<Self, ShaclError> {
        Ok(Self {
            store: Arc::new(OnceLock::new()),
            data: DataBackend::Indexed(IndexedGraph::from_triples(data_triples)),
            shapes_graph,
        })
    }

    /// View over the data graph, independent of the backend.
    pub fn data(&self) -> DataView<'_> {
        match &self.data {
            DataBackend::Graph(graph) => DataView::Plain(graph),
            DataBackend::Indexed(index) => DataView::Indexed(index),
        }
    }

    pub fn store(&self) -> Result<Arc<Store>, ShaclError> {
        if let Some(store) = self.store.get() {
            return Ok(Arc::clone(store));
        }
        let built = Arc::new(Self::build_store(self.data(), &self.shapes_graph)?);
        // Under contention another thread may have won the race; get_or_init
        // returns the stored value either way.
        Ok(Arc::clone(self.store.get_or_init(|| built)))
    }

    fn build_store(data: DataView<'_>, shapes_graph: &Graph) -> Result<Store, ShaclError> {
        let store = Store::new()
            .map_err(|e| ShaclError::Io(format!("Failed to create validation store: {}", e)))?;

        let shapes_graph_name = NamedNodeRef::new_unchecked(SHAPES_GRAPH_IRI);
        let quads = data
            .iter()
            .map(|(subject, predicate, object)| {
                QuadRef::new(subject, predicate, object, GraphNameRef::DefaultGraph)
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

    pub fn shapes_graph(&self) -> &Graph {
        &self.shapes_graph
    }
}
