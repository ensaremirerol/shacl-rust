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
    store: Arc<OnceLock<Result<Arc<Store>, String>>>,
    // Number of times the store has actually been built; must never exceed 1.
    store_builds: std::sync::atomic::AtomicUsize,
    data: DataBackend,
    shapes_graph: Graph,
}

impl ValidationDataset {
    pub fn from_graphs(data_graph: Graph, shapes_graph: Graph) -> Result<Self, ShaclError> {
        Ok(Self {
            store: Arc::new(OnceLock::new()),
            store_builds: std::sync::atomic::AtomicUsize::new(0),
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
            store_builds: std::sync::atomic::AtomicUsize::new(0),
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
            store_builds: std::sync::atomic::AtomicUsize::new(0),
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
        // The build runs inside the OnceLock initializer so concurrent callers
        // block on the one in-flight build instead of each materializing their
        // own copy of the dataset (which multiplies peak memory by the number
        // of racing threads). A build failure is cached too: retrying cannot
        // succeed, since the inputs don't change.
        let result = self.store.get_or_init(|| {
            self.store_builds
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Self::build_store(self.data(), &self.shapes_graph)
                .map(Arc::new)
                .map_err(|e| e.to_string())
        });
        match result {
            Ok(store) => Ok(Arc::clone(store)),
            Err(message) => Err(ShaclError::Io(message.clone())),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::{NamedNode, TripleRef};
    use std::sync::atomic::Ordering;
    use std::sync::Barrier;

    /// Concurrent `store()` callers must not each build their own copy of the
    /// store: the build materializes the full dataset, so N racing builders
    /// multiply peak memory by N (the cause of the era-es-tds 40x blowup).
    #[test]
    fn concurrent_store_calls_build_store_once() {
        let mut data_graph = Graph::new();
        for i in 0..100 {
            let s = NamedNode::new(format!("http://example.org/s{i}")).unwrap();
            let p = NamedNode::new("http://example.org/p").unwrap();
            let o = NamedNode::new(format!("http://example.org/o{i}")).unwrap();
            data_graph.insert(TripleRef::new(&s, &p, &o));
        }
        let dataset = ValidationDataset::from_graphs(data_graph, Graph::new()).unwrap();

        let threads = 16;
        let barrier = Barrier::new(threads);
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    barrier.wait();
                    dataset.store().unwrap();
                });
            }
        });

        assert_eq!(
            dataset.store_builds.load(Ordering::SeqCst),
            1,
            "store must be built exactly once even under concurrent access"
        );
    }
}
