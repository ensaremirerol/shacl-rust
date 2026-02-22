use log::debug;
use oxigraph::model::vocab::rdf::TYPE;
use oxigraph::model::{NamedNodeRef, NamedOrBlankNodeRef, TermRef};
use std::collections::HashSet;
use std::fmt::Display;

/// SHACL Target that represents a target in the SHACL specification.
///
/// ```
/// use shacl_rust::Target;
/// use shacl_rust::rdf::read_graph_from_string;
/// use oxigraph::model::{NamedNodeRef, TermRef};
///
/// let graph_string = r#"
///    @prefix ex: <http://example.org/> .
///    ex:Alice a ex:Person .
///    ex:Alice ex:worksAt ex:CompanyX .
/// "#;
///
/// let graph = read_graph_from_string(graph_string, "turtle").expect("Failed to read graph");
///
/// let alice = NamedNodeRef::new("http://example.org/Alice").unwrap();
/// let person = NamedNodeRef::new("http://example.org/Person").unwrap();
/// let works_at = NamedNodeRef::new("http://example.org/worksAt").unwrap();
/// let company_x = NamedNodeRef::new("http://example.org/CompanyX").unwrap();
///
/// let target_node = Target::Node(TermRef::from(alice));
/// let target_class = Target::Class(person.into());
/// let target_subjects_of = Target::SubjectsOf(works_at);
/// let target_objects_of = Target::ObjectsOf(works_at);
///
/// assert!(target_node.resolve_target_for_given_graph(&graph).contains(&alice.into()));
/// assert!(target_class.resolve_target_for_given_graph(&graph).contains(&alice.into()));
/// assert!(target_subjects_of.resolve_target_for_given_graph(&graph).contains(&alice.into()));
/// assert!(target_objects_of.resolve_target_for_given_graph(&graph).contains(&company_x.into()));
///
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target<'a> {
    Node(TermRef<'a>),
    Class(NamedOrBlankNodeRef<'a>),
    SubjectsOf(NamedNodeRef<'a>),
    ObjectsOf(NamedNodeRef<'a>),
    Advanced(NamedOrBlankNodeRef<'a>),
}

impl<'a> Target<'a> {
    pub fn resolve_target_for_given_graph(
        &self,
        graph: &'a oxigraph::model::Graph,
    ) -> HashSet<oxigraph::model::TermRef<'a>> {
        debug!(
            "Resolving target: {} for graph with {} triples",
            self,
            graph.len()
        );
        match self {
            Target::Node(term) => {
                let mut set = HashSet::new();
                set.insert(*term);
                set
            }
            Target::Class(class) => {
                let mut set = HashSet::new();
                let all_subclasses = crate::utils::collect_all_subclasses(*class, graph);
                for subclass in all_subclasses {
                    graph
                        .subjects_for_predicate_object(TYPE, subclass)
                        .for_each(|instance| {
                            set.insert(TermRef::from(instance));
                        });
                }
                set
            }
            Target::SubjectsOf(property) => {
                let mut set = HashSet::new();
                let all_subproperties = crate::utils::collect_all_subproperties(*property, graph);
                for subproperty in all_subproperties {
                    // Get all subjects where this property is the predicate
                    for triple in graph.triples_for_predicate(subproperty) {
                        set.insert(triple.subject.into());
                    }
                }
                set
            }
            Target::ObjectsOf(property) => {
                let mut set = HashSet::new();
                let all_subproperties = crate::utils::collect_all_subproperties(*property, graph);
                for subproperty in all_subproperties {
                    // Get all objects where this property is the predicate
                    for triple in graph.triples_for_predicate(subproperty) {
                        match triple.object {
                            TermRef::NamedNode(_) | TermRef::BlankNode(_) => {
                                set.insert(triple.object);
                            }
                            TermRef::Literal(_) => {}
                        }
                    }
                }
                set
            }
            Target::Advanced(_) => HashSet::new(),
        }
    }
}

impl<'a> Display for Target<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::Node(node) => write!(f, "sh:targetNode {}", node),
            Target::Class(class) => write!(f, "sh:targetClass {}", class),
            Target::SubjectsOf(property) => write!(f, "sh:targetSubjectsOf {}", property),
            Target::ObjectsOf(property) => write!(f, "sh:targetObjectsOf {}", property),
            Target::Advanced(target) => write!(f, "sh:target {}", target),
        }
    }
}
