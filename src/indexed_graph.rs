//! **Experimental** interned, immutable data-graph index.
//!
//! [`IndexedGraph`] is an alternative backend for the *data* graph during
//! validation. Unlike [`oxigraph::model::Graph`] it is immutable after
//! construction, which allows a much cheaper representation: every RDF term is
//! interned once into an arena and triples become `(u32, u32, u32)` ids over
//! hash indexes tailored to the lookups the validator performs. Construction
//! is sharded across threads on native targets.
//!
//! Semantics match `Graph` where the validator can observe them: duplicate
//! triples are deduplicated and lookups return each result exactly once.
//! Iteration order is unspecified (and differs from `Graph`'s sorted order).
//!
//! This API is experimental and may change or move in any release.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use oxigraph::model::{NamedNodeRef, NamedOrBlankNodeRef, Term, TermRef, Triple};

#[cfg(not(target_family = "wasm"))]
use rayon::prelude::*;

/// One (predicate, object) edge of a subject, borrowed from the index arena.
#[derive(Debug, Clone, Copy)]
pub struct PredicateObjectRef<'a> {
    pub predicate: NamedNodeRef<'a>,
    pub object: TermRef<'a>,
}

/// Ids stored under one term-hash bucket. Collisions on the full 64-bit hash
/// are rare enough that the single-id representation covers almost all keys.
#[derive(Debug)]
enum IdSlot {
    One(u32),
    Many(Vec<u32>),
}

impl IdSlot {
    fn ids(&self) -> &[u32] {
        match self {
            IdSlot::One(id) => std::slice::from_ref(id),
            IdSlot::Many(ids) => ids,
        }
    }

    fn push(&mut self, id: u32) {
        match self {
            IdSlot::One(first) => *self = IdSlot::Many(vec![*first, id]),
            IdSlot::Many(ids) => ids.push(id),
        }
    }
}

/// Experimental interned data-graph index. See the module docs.
#[derive(Debug)]
pub struct IndexedGraph {
    /// Term arena: id -> term. Lookups hand out `TermRef`s borrowed from here.
    terms: Box<[Term]>,
    /// term hash -> interned id(s); resolved by comparing against `terms`.
    ids_by_hash: HashMap<u64, IdSlot>,
    /// Shard s: subject id (s % nshards == shard) -> sorted, deduplicated
    /// (predicate id, object id) pairs.
    by_subject: Vec<HashMap<u32, Vec<(u32, u32)>>>,
    /// Shard s: (predicate id, object id) (object % nshards == shard) ->
    /// sorted, deduplicated subject ids.
    by_predicate_object: Vec<HashMap<(u32, u32), Vec<u32>>>,
    nshards: usize,
    len: usize,
}

fn term_hash(term: TermRef<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    term.hash(&mut hasher);
    hasher.finish()
}

impl IndexedGraph {
    /// Builds the index from a stream of triples, interning as it goes.
    pub fn from_triples(triples: impl IntoIterator<Item = Triple>) -> Self {
        let mut terms: Vec<Term> = Vec::new();
        let mut ids_by_hash: HashMap<u64, IdSlot> = HashMap::new();

        let mut intern = |term: Term, terms: &mut Vec<Term>| -> u32 {
            let hash = term_hash(term.as_ref());
            match ids_by_hash.entry(hash) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    for &id in entry.get().ids() {
                        if terms[id as usize] == term {
                            return id;
                        }
                    }
                    let id = terms.len() as u32;
                    terms.push(term);
                    entry.get_mut().push(id);
                    id
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let id = terms.len() as u32;
                    terms.push(term);
                    entry.insert(IdSlot::One(id));
                    id
                }
            }
        };

        let encoded: Vec<(u32, u32, u32)> = triples
            .into_iter()
            .map(|triple| {
                let s = intern(Term::from(triple.subject), &mut terms);
                let p = intern(Term::from(triple.predicate), &mut terms);
                let o = intern(triple.object, &mut terms);
                (s, p, o)
            })
            .collect();

        #[cfg(not(target_family = "wasm"))]
        let nshards = rayon::current_num_threads().max(1);
        #[cfg(target_family = "wasm")]
        let nshards = 1;

        let build_subject_shard = |shard: usize| {
            let mut map: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
            for &(s, p, o) in &encoded {
                if s as usize % nshards == shard {
                    map.entry(s).or_default().push((p, o));
                }
            }
            for pairs in map.values_mut() {
                pairs.sort_unstable();
                pairs.dedup();
            }
            map
        };
        let build_po_shard = |shard: usize| {
            let mut map: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
            for &(s, p, o) in &encoded {
                if o as usize % nshards == shard {
                    map.entry((p, o)).or_default().push(s);
                }
            }
            for subjects in map.values_mut() {
                subjects.sort_unstable();
                subjects.dedup();
            }
            map
        };

        #[cfg(not(target_family = "wasm"))]
        let (by_subject, by_predicate_object): (Vec<_>, Vec<_>) = rayon::join(
            || {
                (0..nshards)
                    .into_par_iter()
                    .map(build_subject_shard)
                    .collect()
            },
            || (0..nshards).into_par_iter().map(build_po_shard).collect(),
        );
        #[cfg(target_family = "wasm")]
        let (by_subject, by_predicate_object): (Vec<_>, Vec<_>) = (
            (0..nshards).map(build_subject_shard).collect(),
            (0..nshards).map(build_po_shard).collect(),
        );

        let len = by_subject
            .iter()
            .flat_map(|shard| shard.values())
            .map(|pairs| pairs.len())
            .sum();

        IndexedGraph {
            terms: terms.into_boxed_slice(),
            ids_by_hash,
            by_subject,
            by_predicate_object,
            nshards,
            len,
        }
    }

    /// Builds the index from an existing graph (convenience for tests and the
    /// experimental [`ValidationDataset`](crate::validation::dataset::ValidationDataset) constructor).
    pub fn from_graph(graph: &oxigraph::model::Graph) -> Self {
        Self::from_triples(graph.iter().map(Triple::from))
    }

    /// Number of distinct triples.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn term_id(&self, term: TermRef<'_>) -> Option<u32> {
        let slot = self.ids_by_hash.get(&term_hash(term))?;
        slot.ids()
            .iter()
            .copied()
            .find(|&id| self.terms[id as usize].as_ref() == term)
    }

    fn term_ref(&self, id: u32) -> TermRef<'_> {
        self.terms[id as usize].as_ref()
    }

    fn named_node_ref(&self, id: u32) -> NamedNodeRef<'_> {
        match &self.terms[id as usize] {
            Term::NamedNode(n) => n.as_ref(),
            other => unreachable!("predicate id {id} is not an IRI: {other}"),
        }
    }

    fn named_or_blank_ref(&self, id: u32) -> Option<NamedOrBlankNodeRef<'_>> {
        match &self.terms[id as usize] {
            Term::NamedNode(n) => Some(NamedOrBlankNodeRef::NamedNode(n.as_ref())),
            Term::BlankNode(b) => Some(NamedOrBlankNodeRef::BlankNode(b.as_ref())),
            _ => None,
        }
    }

    fn subject_pairs(&self, subject: NamedOrBlankNodeRef<'_>) -> &[(u32, u32)] {
        let Some(s) = self.term_id(TermRef::from(subject)) else {
            return &[];
        };
        self.by_subject[s as usize % self.nshards]
            .get(&s)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn objects_for_subject_predicate<'a, 'b>(
        &'a self,
        subject: NamedOrBlankNodeRef<'b>,
        predicate: NamedNodeRef<'b>,
    ) -> impl Iterator<Item = TermRef<'a>> + use<'a> {
        let p = self.term_id(TermRef::from(predicate));
        self.subject_pairs(subject)
            .iter()
            .filter(move |&&(pair_p, _)| Some(pair_p) == p)
            .map(move |&(_, o)| self.term_ref(o))
    }

    pub fn object_for_subject_predicate<'a, 'b>(
        &'a self,
        subject: NamedOrBlankNodeRef<'b>,
        predicate: NamedNodeRef<'b>,
    ) -> Option<TermRef<'a>> {
        self.objects_for_subject_predicate(subject, predicate)
            .next()
    }

    pub fn subjects_for_predicate_object<'a, 'b>(
        &'a self,
        predicate: NamedNodeRef<'b>,
        object: TermRef<'b>,
    ) -> impl Iterator<Item = NamedOrBlankNodeRef<'a>> + use<'a> {
        let p = self.term_id(TermRef::from(predicate));
        let o = self.term_id(object);
        let subjects: &[u32] = match (p, o) {
            (Some(p), Some(o)) => self.by_predicate_object[o as usize % self.nshards]
                .get(&(p, o))
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            _ => &[],
        };
        subjects
            .iter()
            .filter_map(move |&s| self.named_or_blank_ref(s))
    }

    pub fn triples_for_subject<'a, 'b>(
        &'a self,
        subject: NamedOrBlankNodeRef<'b>,
    ) -> impl Iterator<Item = PredicateObjectRef<'a>> + use<'a> {
        self.subject_pairs(subject)
            .iter()
            .map(move |&(p, o)| PredicateObjectRef {
                predicate: self.named_node_ref(p),
                object: self.term_ref(o),
            })
    }

    /// Iterates all (subject, object) pairs with the given predicate. This is
    /// a full scan; it backs target resolution, which runs once per target.
    pub fn triples_for_predicate<'a, 'b>(
        &'a self,
        predicate: NamedNodeRef<'b>,
    ) -> impl Iterator<Item = (NamedOrBlankNodeRef<'a>, TermRef<'a>)> + use<'a> {
        let p = self.term_id(TermRef::from(predicate));
        self.by_subject.iter().flat_map(move |shard| {
            shard.iter().flat_map(move |(&s, pairs)| {
                pairs
                    .iter()
                    .filter(move |&&(pair_p, _)| Some(pair_p) == p)
                    .filter_map(move |&(_, o)| {
                        Some((self.named_or_blank_ref(s)?, self.term_ref(o)))
                    })
            })
        })
    }

    /// Iterates every triple. Order is unspecified.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (NamedOrBlankNodeRef<'_>, NamedNodeRef<'_>, TermRef<'_>)> {
        self.by_subject.iter().flat_map(move |shard| {
            shard.iter().flat_map(move |(&s, pairs)| {
                let subject = self
                    .named_or_blank_ref(s)
                    .expect("subject id is not an IRI or blank node");
                pairs
                    .iter()
                    .map(move |&(p, o)| (subject, self.named_node_ref(p), self.term_ref(o)))
            })
        })
    }
}

/// Copyable view over either data-graph backend, exposing exactly the lookup
/// surface the validator uses. Obtained from
/// [`ValidationDataset::data`](crate::validation::dataset::ValidationDataset::data),
/// or via `From<&Graph>` for plain graphs.
#[derive(Debug, Clone, Copy)]
pub enum DataView<'a> {
    Plain(&'a oxigraph::model::Graph),
    Indexed(&'a IndexedGraph),
}

impl<'a> From<&'a oxigraph::model::Graph> for DataView<'a> {
    fn from(graph: &'a oxigraph::model::Graph) -> Self {
        DataView::Plain(graph)
    }
}

impl<'a> From<&'a IndexedGraph> for DataView<'a> {
    fn from(index: &'a IndexedGraph) -> Self {
        DataView::Indexed(index)
    }
}

/// Iterator over one of two concrete iterator types; used so [`DataView`]
/// methods stay allocation-free.
pub enum EitherIter<A, B> {
    A(A),
    B(B),
}

impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for EitherIter<A, B> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self {
            EitherIter::A(a) => a.next(),
            EitherIter::B(b) => b.next(),
        }
    }
}

impl<'a> DataView<'a> {
    pub fn len(self) -> usize {
        match self {
            DataView::Plain(g) => g.len(),
            DataView::Indexed(ix) => ix.len(),
        }
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn objects_for_subject_predicate(
        self,
        subject: NamedOrBlankNodeRef<'a>,
        predicate: NamedNodeRef<'a>,
    ) -> impl Iterator<Item = TermRef<'a>> {
        match self {
            DataView::Plain(g) => {
                EitherIter::A(g.objects_for_subject_predicate(subject, predicate))
            }
            DataView::Indexed(ix) => {
                EitherIter::B(ix.objects_for_subject_predicate(subject, predicate))
            }
        }
    }

    pub fn object_for_subject_predicate(
        self,
        subject: NamedOrBlankNodeRef<'a>,
        predicate: NamedNodeRef<'a>,
    ) -> Option<TermRef<'a>> {
        self.objects_for_subject_predicate(subject, predicate)
            .next()
    }

    pub fn subjects_for_predicate_object(
        self,
        predicate: NamedNodeRef<'a>,
        object: TermRef<'a>,
    ) -> impl Iterator<Item = NamedOrBlankNodeRef<'a>> {
        match self {
            DataView::Plain(g) => EitherIter::A(g.subjects_for_predicate_object(predicate, object)),
            DataView::Indexed(ix) => {
                EitherIter::B(ix.subjects_for_predicate_object(predicate, object))
            }
        }
    }

    pub fn triples_for_subject(
        self,
        subject: NamedOrBlankNodeRef<'a>,
    ) -> impl Iterator<Item = PredicateObjectRef<'a>> {
        match self {
            DataView::Plain(g) => {
                EitherIter::A(g.triples_for_subject(subject).map(|t| PredicateObjectRef {
                    predicate: t.predicate,
                    object: t.object,
                }))
            }
            DataView::Indexed(ix) => EitherIter::B(ix.triples_for_subject(subject)),
        }
    }

    /// Iterates all (subject, object) pairs with the given predicate.
    pub fn triples_for_predicate(
        self,
        predicate: NamedNodeRef<'a>,
    ) -> impl Iterator<Item = (NamedOrBlankNodeRef<'a>, TermRef<'a>)> {
        match self {
            DataView::Plain(g) => EitherIter::A(
                g.triples_for_predicate(predicate)
                    .map(|t| (t.subject, t.object)),
            ),
            DataView::Indexed(ix) => EitherIter::B(ix.triples_for_predicate(predicate)),
        }
    }

    /// Iterates every triple as (subject, predicate, object) refs.
    pub fn iter(
        self,
    ) -> impl Iterator<Item = (NamedOrBlankNodeRef<'a>, NamedNodeRef<'a>, TermRef<'a>)> {
        match self {
            DataView::Plain(g) => {
                EitherIter::A(g.iter().map(|t| (t.subject, t.predicate, t.object)))
            }
            DataView::Indexed(ix) => EitherIter::B(ix.iter()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdf::read_graph_from_string;

    const TTL: &str = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Person ; ex:knows ex:b, ex:c ; ex:name "A" .
        ex:b a ex:Person ; ex:knows ex:a .
        ex:b ex:knows ex:a .
        _:x a ex:Person ; ex:knows ex:a .
    "#;

    fn both() -> (oxigraph::model::Graph, IndexedGraph) {
        let g = read_graph_from_string(TTL, "turtle").unwrap();
        let ix = IndexedGraph::from_graph(&g);
        (g, ix)
    }

    #[test]
    fn len_matches_graph_dedup() {
        let (g, ix) = both();
        assert_eq!(g.len(), ix.len());
    }

    #[test]
    fn objects_match_graph() {
        let (g, ix) = both();
        let a = NamedNodeRef::new("http://example.org/a").unwrap();
        let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
        let mut from_g: Vec<String> = g
            .objects_for_subject_predicate(NamedOrBlankNodeRef::from(a), knows)
            .map(|t| t.to_string())
            .collect();
        let mut from_ix: Vec<String> = ix
            .objects_for_subject_predicate(NamedOrBlankNodeRef::from(a), knows)
            .map(|t| t.to_string())
            .collect();
        from_g.sort();
        from_ix.sort();
        assert_eq!(from_g, from_ix);
        assert_eq!(from_ix.len(), 2);
    }

    #[test]
    fn subjects_match_graph_including_blank() {
        let (g, ix) = both();
        let ty = oxigraph::model::vocab::rdf::TYPE;
        let person = NamedNodeRef::new("http://example.org/Person").unwrap();
        let mut from_g: Vec<String> = g
            .subjects_for_predicate_object(ty, TermRef::from(person))
            .map(|s| s.to_string())
            .collect();
        let mut from_ix: Vec<String> = ix
            .subjects_for_predicate_object(ty, TermRef::from(person))
            .map(|s| s.to_string())
            .collect();
        from_g.sort();
        from_ix.sort();
        assert_eq!(from_g, from_ix);
        assert_eq!(from_ix.len(), 3);
    }

    #[test]
    fn duplicate_triples_deduplicated_in_lookups() {
        let (_, ix) = both();
        let b = NamedNodeRef::new("http://example.org/b").unwrap();
        let knows = NamedNodeRef::new("http://example.org/knows").unwrap();
        assert_eq!(
            ix.objects_for_subject_predicate(NamedOrBlankNodeRef::from(b), knows)
                .count(),
            1
        );
    }

    #[test]
    fn missing_terms_return_empty() {
        let (_, ix) = both();
        let nope = NamedNodeRef::new("http://example.org/missing").unwrap();
        assert_eq!(
            ix.objects_for_subject_predicate(NamedOrBlankNodeRef::from(nope), nope)
                .count(),
            0
        );
        assert_eq!(
            ix.subjects_for_predicate_object(nope, TermRef::from(nope))
                .count(),
            0
        );
        assert_eq!(
            ix.triples_for_subject(NamedOrBlankNodeRef::from(nope))
                .count(),
            0
        );
    }

    #[test]
    fn iter_covers_all_triples() {
        let (g, ix) = both();
        assert_eq!(ix.iter().count(), g.len());
    }
}
