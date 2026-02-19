use std::{collections::HashSet, fmt::Display};

use oxigraph::model::{NamedNodeRef, NamedOrBlankNodeRef, TermRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathElement<'a> {
    Iri(NamedNodeRef<'a>),
    Inverse(NamedNodeRef<'a>),
    ZeroOrMore(Box<PathElement<'a>>),
    OneOrMore(Box<PathElement<'a>>),
    ZeroOrOne(Box<PathElement<'a>>),
    Alternative(Vec<PathElement<'a>>),
}

/// SHACL Path
/// ```
/// use shacl_rust::{Path, PathElement};
/// use shacl_rust::rdf::read_graph_from_string;
/// use oxigraph::model::{NamedNodeRef, NamedOrBlankNodeRef};
///
/// let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
/// let path_loopback = Path::new()
///    .add_element(PathElement::Iri(knows))
///    .add_element(PathElement::Inverse(knows));
/// let path_single = Path::new().add_element(PathElement::Iri(knows));
///
/// let zero_or_more_path = Path::new()
///     .add_element(PathElement::ZeroOrMore(Box::new(PathElement::Iri(knows))));
///
/// let graph_string = r#"
///     @prefix ex: <http://example.org/> .
///     ex:Alice ex:knows ex:Bob .
///     ex:Bob ex:knows ex:Charlie .
///     ex:Charlie ex:knows ex:Alice .
/// "#;
/// let graph = read_graph_from_string(graph_string, "turtle").expect("Failed to read graph");
/// let alice = NamedOrBlankNodeRef::from(NamedNodeRef::new("http://example.org/Alice").unwrap());
///
/// let results_loopback = path_loopback.resolve_path_for_given_node(&graph, &alice);
/// assert_eq!(results_loopback.len(), 1);
/// assert_eq!(results_loopback[0], NamedNodeRef::new("http://example.org/Alice").unwrap().into());
///
/// let results_single = path_single.resolve_path_for_given_node(&graph, &alice);
/// assert_eq!(results_single.len(), 1);
/// assert_eq!(results_single[0], NamedNodeRef::new("http://example.org/Bob").unwrap().into());
///
/// let results_zero_or_more = zero_or_more_path.resolve_path_for_given_node(&graph, &alice);
/// assert_eq!(results_zero_or_more.len(), 3);
/// assert!(results_zero_or_more.contains(&NamedNodeRef::new("http://example.org/Alice").unwrap().into()));
/// assert!(results_zero_or_more.contains(&NamedNodeRef::new("http://example.org/Bob").unwrap().into()));
/// assert!(results_zero_or_more.contains(&NamedNodeRef::new("http://example.org/Charlie").unwrap().into()));
/// ```
///
///
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Path<'a> {
    source: Option<NamedOrBlankNodeRef<'a>>,
    path: Vec<PathElement<'a>>,
}

impl<'a> Path<'a> {
    pub fn new() -> Self {
        Path {
            path: Vec::new(),
            source: None,
        }
    }

    pub fn add_element(mut self, element: PathElement<'a>) -> Self {
        self.path.push(element);
        self
    }

    pub fn set_source(mut self, source: NamedOrBlankNodeRef<'a>) -> Self {
        self.source = Some(source);
        self
    }

    pub fn get_elements(&self) -> &[PathElement<'a>] {
        &self.path
    }

    pub fn get_source(&self) -> Option<NamedOrBlankNodeRef<'a>> {
        self.source
    }

    pub fn resolve_path_for_given_node(
        &self,
        graph: &'a oxigraph::model::Graph,
        node: &oxigraph::model::NamedOrBlankNodeRef<'a>,
    ) -> Vec<oxigraph::model::TermRef<'a>> {
        // Start with the given node as the initial set
        let mut current_nodes: Vec<TermRef<'a>> = vec![(*node).into()];

        // Apply each path element in sequence
        for element in &self.path {
            current_nodes = self.resolve_element(graph, element, &current_nodes);
        }

        current_nodes
    }

    /// Resolves a single path element for a set of nodes
    fn resolve_element(
        &self,
        graph: &'a oxigraph::model::Graph,
        element: &PathElement<'a>,
        nodes: &[TermRef<'a>],
    ) -> Vec<TermRef<'a>> {
        let mut results = Vec::new();
        let subjects: Vec<NamedOrBlankNodeRef<'a>> = nodes
            .iter()
            .filter_map(|node| match node {
                TermRef::NamedNode(n) => Some(NamedOrBlankNodeRef::from(*n)),
                TermRef::BlankNode(b) => Some(NamedOrBlankNodeRef::from(*b)),
                TermRef::Literal(_) => None,
            })
            .collect();
        for subject in subjects {
            match element {
                PathElement::Iri(predicate) => {
                    for triple in graph {
                        if triple.subject == subject && triple.predicate == (*predicate) {
                            results.push(triple.object);
                        }
                    }
                }
                PathElement::Inverse(predicate) => {
                    // Inverse property: find all subjects where node is object
                    for triple in graph {
                        if triple.object == subject.into() && triple.predicate == (*predicate) {
                            results.push(triple.subject.into());
                        }
                    }
                }
                PathElement::ZeroOrMore(path_element) => {
                    // Transitive closure including the starting node (Kleene star)
                    results.push(subject.into());
                    let mut visited: HashSet<TermRef<'a>> = HashSet::new();
                    visited.insert(subject.into());
                    let mut to_visit: Vec<TermRef<'a>> = vec![subject.into()];

                    while let Some(current) = to_visit.pop() {
                        // Get next nodes by applying the path element
                        let next_nodes = self.resolve_element(graph, path_element, &[current]);
                        for next in next_nodes {
                            if visited.insert(next) {
                                results.push(next);
                                to_visit.push(next);
                            }
                        }
                    }
                }
                PathElement::OneOrMore(path_element) => {
                    // Transitive closure, not including the starting node (Kleene plus)
                    let mut visited: HashSet<TermRef<'a>> = HashSet::new();
                    visited.insert(subject.into());
                    let mut to_visit: Vec<TermRef<'a>> = vec![subject.into()];

                    while let Some(current) = to_visit.pop() {
                        // Get next nodes by applying the path element
                        let next_nodes = self.resolve_element(graph, path_element, &[current]);
                        for next in next_nodes {
                            if visited.insert(next) {
                                results.push(next);
                                to_visit.push(next);
                            }
                        }
                    }
                }
                PathElement::ZeroOrOne(path_element) => {
                    // Optional path: include the node itself and direct neighbors
                    results.push(subject.into());

                    let next_nodes = self.resolve_element(graph, path_element, &[subject.into()]);
                    results.extend(next_nodes);
                }
                PathElement::Alternative(alternatives) => {
                    // Apply all alternatives and merge results
                    for alt in alternatives {
                        results.extend(self.resolve_element(graph, alt, &[subject.into()]));
                    }
                }
            }
        }

        // Remove duplicates
        let mut unique_results = HashSet::new();
        results
            .into_iter()
            .filter(|r| unique_results.insert(*r))
            .collect()
    }
}

impl Display for PathElement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathElement::Iri(iri) => write!(f, "{}", iri),
            PathElement::Inverse(iri) => write!(f, "^{}", iri),
            PathElement::ZeroOrMore(e) => write!(f, "({}*)", e),
            PathElement::OneOrMore(e) => write!(f, "({}+)", e),
            PathElement::ZeroOrOne(e) => write!(f, "({}?)", e),
            PathElement::Alternative(alts) => {
                let alt_strs: Vec<String> = alts.iter().map(|alt| format!("{}", alt)).collect();
                write!(f, "({})", alt_strs.join(" | "))
            }
        }
    }
}

impl Display for Path<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path_str = self
            .path
            .iter()
            .map(|element| format!("{}", element))
            .collect::<Vec<String>>()
            .join(" / ");
        write!(f, "{}", path_str)
    }
}
