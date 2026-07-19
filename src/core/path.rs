use std::{collections::HashSet, fmt::Display};

use log::debug;
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
/// let works_for = NamedNodeRef::new("http://example.org/worksFor").unwrap();
/// let path_loopback = Path::new()
///    .add_element(PathElement::Iri(knows))
///    .add_element(PathElement::Inverse(knows));
/// let path_single = Path::new().add_element(PathElement::Iri(knows));
///
/// let zero_or_more_path = Path::new()
///     .add_element(PathElement::ZeroOrMore(Box::new(PathElement::Iri(knows))));
///
/// let complex_path = Path::new().add_element(PathElement::ZeroOrMore(Box::new(PathElement::Alternative(vec![
///    PathElement::Iri(knows), PathElement::Iri(works_for)
/// ]))));
///
/// let graph_string = r#"
///     @prefix ex: <http://example.org/> .
///     ex:Alice ex:knows ex:Bob .
///     ex:Bob ex:knows ex:Charlie .
///     ex:Charlie ex:knows ex:Alice .
///     ex:Charlie ex:worksFor ex:Daniel .
///     ex:Daniel ex:knows ex:David .
///
/// "#;
/// let graph = read_graph_from_string(graph_string, "turtle").expect("Failed to read graph");
/// let alice = NamedOrBlankNodeRef::from(NamedNodeRef::new("http://example.org/Alice").unwrap());
///
/// let results_loopback = path_loopback.resolve_path_for_given_node(&graph, &alice);
/// println!("Loopback results: {:?}", results_loopback);
/// assert_eq!(results_loopback.len(), 1);
/// assert_eq!(results_loopback[0], NamedNodeRef::new("http://example.org/Alice").unwrap().into());
///
/// let results_single = path_single.resolve_path_for_given_node(&graph, &alice);
/// println!("Single step results: {:?}", results_single);
/// assert_eq!(results_single.len(), 1);
/// assert_eq!(results_single[0], NamedNodeRef::new("http://example.org/Bob").unwrap().into());
///
/// let results_zero_or_more = zero_or_more_path.resolve_path_for_given_node(&graph, &alice);
/// println!("Zero or more results: {:?}", results_zero_or_more);
/// assert_eq!(results_zero_or_more.len(), 3);
/// assert!(results_zero_or_more.contains(&NamedNodeRef::new("http://example.org/Alice").unwrap().into()));
/// assert!(results_zero_or_more.contains(&NamedNodeRef::new("http://example.org/Bob").unwrap().into()));
/// assert!(results_zero_or_more.contains(&NamedNodeRef::new("http://example.org/Charlie").unwrap().into()));
///
/// let results_complex = complex_path.resolve_path_for_given_node(&graph, &alice);
/// println!("Complex path results: {:?}", results_complex);
/// assert_eq!(results_complex.len(), 5);
/// assert!(results_complex.contains(&NamedNodeRef::new("http://example.org/Alice").unwrap().into()));
/// assert!(results_complex.contains(&NamedNodeRef::new("http://example.org/Bob").unwrap().into()));
/// assert!(results_complex.contains(&NamedNodeRef::new("http://example.org/Charlie").unwrap().into()));
/// assert!(results_complex.contains(&NamedNodeRef::new("http://example.org/Daniel").unwrap().into()));
/// assert!(results_complex.contains(&NamedNodeRef::new("http://example.org/David").unwrap().into()));
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

    /// Resolves the path for a given node in the graph, returning all reachable nodes.
    pub fn resolve_path_for_given_node(
        &self,
        graph: &'a oxigraph::model::Graph,
        node: &oxigraph::model::NamedOrBlankNodeRef<'a>,
    ) -> Vec<oxigraph::model::TermRef<'a>> {
        debug!("Resolving path for node {:?} with path: {}", node, self);
        let mut current_nodes: Vec<TermRef<'a>> = vec![(*node).into()];

        // Apply each path element in sequence, deduplicating the frontier per step.
        for element in &self.path {
            let mut next_nodes: Vec<TermRef<'a>> = Vec::new();
            let mut seen: HashSet<TermRef<'a>> = HashSet::new();
            for current in &current_nodes {
                let subject = match current {
                    TermRef::NamedNode(n) => NamedOrBlankNodeRef::from(*n),
                    TermRef::BlankNode(b) => NamedOrBlankNodeRef::from(*b),
                    TermRef::Literal(_) => continue,
                };
                Self::expand(graph, element, subject, &mut |term| {
                    if seen.insert(term) {
                        next_nodes.push(term);
                    }
                });
            }
            current_nodes = next_nodes;
        }
        debug!("Resolved nodes: {:?}", current_nodes);
        current_nodes
    }

    /// Emits every node reachable from `subject` via `element`. May emit
    /// duplicates; callers deduplicate.
    fn expand(
        graph: &'a oxigraph::model::Graph,
        element: &PathElement<'a>,
        subject: NamedOrBlankNodeRef<'a>,
        emit: &mut dyn FnMut(TermRef<'a>),
    ) {
        match element {
            PathElement::Iri(predicate) => {
                for object in graph.objects_for_subject_predicate(subject, *predicate) {
                    emit(object);
                }
            }
            PathElement::Inverse(predicate) => {
                for s in graph.subjects_for_predicate_object(*predicate, TermRef::from(subject)) {
                    emit(TermRef::from(s));
                }
            }
            PathElement::ZeroOrMore(inner) => {
                // Kleene star: include the starting node itself.
                emit(subject.into());
                Self::expand_transitive(graph, inner, subject, emit);
            }
            PathElement::OneOrMore(inner) => {
                Self::expand_transitive(graph, inner, subject, emit);
            }
            PathElement::ZeroOrOne(inner) => {
                emit(subject.into());
                Self::expand(graph, inner, subject, emit);
            }
            PathElement::Alternative(alternatives) => {
                for alt in alternatives {
                    Self::expand(graph, alt, subject, emit);
                }
            }
        }
    }

    /// Transitive closure of `element` starting at `start`, excluding `start`
    /// itself. Each reachable node is emitted exactly once.
    fn expand_transitive(
        graph: &'a oxigraph::model::Graph,
        element: &PathElement<'a>,
        start: NamedOrBlankNodeRef<'a>,
        emit: &mut dyn FnMut(TermRef<'a>),
    ) {
        let mut visited: HashSet<TermRef<'a>> = HashSet::new();
        visited.insert(start.into());
        let mut to_visit: Vec<NamedOrBlankNodeRef<'a>> = vec![start];

        while let Some(current) = to_visit.pop() {
            let mut found: Vec<TermRef<'a>> = Vec::new();
            Self::expand(graph, element, current, &mut |term| found.push(term));
            for next in found {
                if visited.insert(next) {
                    emit(next);
                    match next {
                        TermRef::NamedNode(n) => to_visit.push(NamedOrBlankNodeRef::from(n)),
                        TermRef::BlankNode(b) => to_visit.push(NamedOrBlankNodeRef::from(b)),
                        TermRef::Literal(_) => {}
                    }
                }
            }
        }
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
