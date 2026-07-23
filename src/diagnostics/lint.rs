//! Static checks over a parsed shapes graph, independent of any data graph:
//! malformed shapes (missing `sh:path`, unparseable regex/SPARQL), likely
//! typos in the `sh:` namespace, contradictory or dead constraints, and
//! shapes that can never be evaluated. See "Lint rules v1" in
//! docs/superpowers/specs/2026-07-23-diagnostics-design.md for the spec
//! table this module implements (L0001-L0012).

use std::collections::HashSet;

use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};
use regex::Regex;
use spargebra::SparqlParser;

use crate::utils::{
    literal_is_ill_formed, local_name_from_iri, parse_rdf_list, parse_shacl_prefixes,
    term_to_named_or_blank, to_comparable,
};
use crate::vocab::sh;
use crate::{Constraint, Shape};

use super::derive::{build_shapes_snippet, first_sentence};
use super::registry;
use super::{sort_diagnostics, Diagnostic, DiagnosticSeverity};

const SH_NS: &str = "http://www.w3.org/ns/shacl#";

/// Lints `shapes_graph`/`shapes` against the 12 rules in the spec table,
/// returning deterministically sorted diagnostics (empty when clean).
pub fn lint_shapes<'a>(shapes_graph: &'a Graph, shapes: &'a [Shape<'a>]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    lint_l0001_missing_path(shapes_graph, &mut diags);
    lint_l0002_bad_pattern_regex(shapes_graph, &mut diags);
    lint_l0003_bad_sparql(shapes_graph, &mut diags);
    lint_l0004_unknown_term(shapes_graph, &mut diags);
    lint_l0005_path_required(shapes_graph, shapes, &mut diags);
    lint_l0006_unsatisfiable_bounds(shapes_graph, shapes, &mut diags);
    lint_l0007_wrong_term_kind(shapes_graph, &mut diags);
    lint_l0008_empty_list(shapes_graph, &mut diags);
    lint_l0009_noncomparable_bound(shapes_graph, shapes, &mut diags);
    lint_l0010_ignored_without_closed(shapes_graph, &mut diags);
    lint_l0011_dead_shape(shapes_graph, shapes, &mut diags);
    lint_l0012_deactivated(shapes_graph, shapes, &mut diags);
    sort_diagnostics(&mut diags);
    diags
}

/// A fresh [`Diagnostic`] for `code`, with title/help pre-filled from the
/// registry (help is the explanation's first sentence; individual rules may
/// override it, e.g. L0004's did-you-mean suggestion).
fn base_diagnostic(code: &'static str, severity: DiagnosticSeverity) -> Diagnostic {
    let reg_entry = registry::entry(code);
    Diagnostic {
        code,
        severity,
        title: reg_entry.map(|e| e.title.to_string()).unwrap_or_default(),
        constraint_component: None,
        snippets: Vec::new(),
        expected: None,
        actual: None,
        notes: Vec::new(),
        help: reg_entry.map(|e| first_sentence(e.explanation).to_string()),
        focus_node: None,
        source_shape: None,
        path: None,
        verdict: None,
    }
}

/// Pushes a lint diagnostic quoting `node`'s triples in the shapes graph,
/// with `keyword` (a predicate local name, e.g. `"pattern"`) selecting which
/// line gets highlighted.
#[allow(clippy::too_many_arguments)]
fn push_diag<'a>(
    diags: &mut Vec<Diagnostic>,
    graph: &'a Graph,
    code: &'static str,
    severity: DiagnosticSeverity,
    node: NamedOrBlankNodeRef<'a>,
    keyword: Option<&str>,
    actual: Option<String>,
) {
    let mut diag = base_diagnostic(code, severity);
    let (snippet, _bound) = build_shapes_snippet(graph, node, keyword);
    if let Some(snippet) = snippet {
        diag.snippets.push(snippet);
    }
    diag.source_shape = Some(node.to_string());
    diag.actual = actual;
    diags.push(diag);
}

/// Every parsed shape reachable from `shapes`: the top-level shapes plus
/// their nested property shapes (recursively). Shapes nested inside
/// sh:and/sh:or/sh:xone/sh:not/sh:node are not part of this model's
/// `property_shapes` chain and are out of scope for the constraint-level
/// lints (L0005/L0006/L0008/L0009/L0012), matching how `derive.rs`'s
/// `find_owning_shape` already treats shape nesting.
fn walk_shapes<'a>(shapes: &'a [Shape<'a>]) -> Vec<&'a Shape<'a>> {
    let mut all = Vec::new();
    for shape in shapes {
        all.push(shape);
        all.extend(shape.all_nested_shapes());
    }
    all
}

/// L0001 (Error): any object of `sh:property` (named or blank) that has no
/// `sh:path` - a malformed property shape validation silently skips.
fn lint_l0001_missing_path(graph: &Graph, diags: &mut Vec<Diagnostic>) {
    let mut seen = HashSet::new();
    for triple in graph.triples_for_predicate(sh::PROPERTY) {
        let Some(node) = term_to_named_or_blank(triple.object) else {
            continue;
        };
        if graph.object_for_subject_predicate(node, sh::PATH).is_some() {
            continue;
        }
        if !seen.insert(node) {
            continue;
        }
        push_diag(
            diags,
            graph,
            "L0001",
            DiagnosticSeverity::Error,
            node,
            None,
            None,
        );
    }
}

/// The same flag-to-regex translation as `PatternConstraint::new`, so this
/// lint's compile check matches what validation actually runs.
fn translate_pattern_flags(pattern: &str, flags: Option<&str>) -> String {
    match flags {
        Some(f) => {
            let mut with_flags = String::from("(?");
            if f.contains('i') {
                with_flags.push('i');
            }
            if f.contains('m') {
                with_flags.push('m');
            }
            if f.contains('s') {
                with_flags.push('s');
            }
            with_flags.push(')');
            with_flags.push_str(pattern);
            with_flags
        }
        None => pattern.to_string(),
    }
}

/// L0002 (Error): every `sh:pattern` literal in the graph that does not
/// compile as a regex, including its `sh:flags` translation.
fn lint_l0002_bad_pattern_regex(graph: &Graph, diags: &mut Vec<Diagnostic>) {
    for triple in graph.triples_for_predicate(sh::PATTERN) {
        let TermRef::Literal(lit) = triple.object else {
            continue;
        };
        let flags = graph
            .object_for_subject_predicate(triple.subject, sh::FLAGS)
            .and_then(|f| match f {
                TermRef::Literal(l) => Some(l.value().to_string()),
                _ => None,
            });
        let regex_pattern = translate_pattern_flags(lit.value(), flags.as_deref());
        if let Err(err) = Regex::new(&regex_pattern) {
            push_diag(
                diags,
                graph,
                "L0002",
                DiagnosticSeverity::Error,
                triple.subject,
                Some("pattern"),
                Some(err.to_string()),
            );
        }
    }
}

/// L0003 (Error): every `sh:select`/`sh:ask` literal that fails to parse as
/// SPARQL, honoring the executable's own `sh:prefixes`.
fn lint_l0003_bad_sparql(graph: &Graph, diags: &mut Vec<Diagnostic>) {
    for (predicate, keyword) in [(sh::SELECT, "select"), (sh::ASK, "ask")] {
        for triple in graph.triples_for_predicate(predicate) {
            let TermRef::Literal(lit) = triple.object else {
                continue;
            };
            let prefixes = parse_shacl_prefixes(graph, triple.subject);
            let mut parser = SparqlParser::new();
            for (prefix, namespace) in &prefixes {
                if let Ok(with_prefix) = parser
                    .clone()
                    .with_prefix(prefix.clone(), namespace.clone())
                {
                    parser = with_prefix;
                }
            }
            if let Err(err) = parser.parse_query(lit.value()) {
                push_diag(
                    diags,
                    graph,
                    "L0003",
                    DiagnosticSeverity::Error,
                    triple.subject,
                    Some(keyword),
                    Some(err.to_string()),
                );
            }
        }
    }
}

/// A small DP Levenshtein distance, used by L0004's did-you-mean.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

/// The closest whitelisted `sh:` local name to `unknown_local` within edit
/// distance 2, if any (ties broken by whichever is found first).
fn did_you_mean(unknown_local: &str) -> Option<String> {
    let mut best: Option<(String, usize)> = None;
    for term in sh::all_terms() {
        let Some(local) = local_name_from_iri(term.as_str()) else {
            continue;
        };
        let distance = levenshtein(unknown_local, &local);
        if distance <= 2 && best.as_ref().is_none_or(|(_, best_d)| distance < *best_d) {
            best = Some((local, distance));
        }
    }
    best.map(|(local, _)| local)
}

/// L0004 (Warning): any predicate in the `sh:` namespace that is not one of
/// the terms `sh::all_terms()` whitelists, with a did-you-mean suggestion
/// when a known term is within edit distance 2.
fn lint_l0004_unknown_term(graph: &Graph, diags: &mut Vec<Diagnostic>) {
    let known: HashSet<&str> = sh::all_terms().iter().map(|n| n.as_str()).collect();
    let mut seen = HashSet::new();
    for triple in graph.iter() {
        let iri = triple.predicate.as_str();
        if !iri.starts_with(SH_NS) || known.contains(iri) {
            continue;
        }
        if !seen.insert((triple.subject, triple.predicate)) {
            continue;
        }
        let local = local_name_from_iri(iri).unwrap_or_default();
        let mut diag = base_diagnostic("L0004", DiagnosticSeverity::Warning);
        let (snippet, _bound) = build_shapes_snippet(graph, triple.subject, Some(&local));
        if let Some(snippet) = snippet {
            diag.snippets.push(snippet);
        }
        diag.source_shape = Some(triple.subject.to_string());
        diag.actual = Some(format!("sh:{}", local));
        if let Some(suggestion) = did_you_mean(&local) {
            diag.help = Some(format!("did you mean sh:{}?", suggestion));
        }
        diags.push(diag);
    }
}

/// L0005 (Warning): a path-requiring constraint (`Constraint::requires_path`)
/// attached directly to a node shape (`path: None`).
fn lint_l0005_path_required<'a>(
    graph: &'a Graph,
    shapes: &'a [Shape<'a>],
    diags: &mut Vec<Diagnostic>,
) {
    for shape in walk_shapes(shapes) {
        if shape.path.is_some() {
            continue;
        }
        for constraint in &shape.constraints {
            if constraint.requires_path() {
                push_diag(
                    diags,
                    graph,
                    "L0005",
                    DiagnosticSeverity::Warning,
                    shape.node,
                    None,
                    Some(constraint.to_string()),
                );
            }
        }
    }
}

fn constraint_min_count(shape: &Shape) -> Option<i32> {
    shape.constraints.iter().find_map(|c| match c {
        Constraint::MinCount(m) => Some(m.0),
        _ => None,
    })
}

fn constraint_max_count(shape: &Shape) -> Option<i32> {
    shape.constraints.iter().find_map(|c| match c {
        Constraint::MaxCount(m) => Some(m.0),
        _ => None,
    })
}

fn constraint_min_length(shape: &Shape) -> Option<i32> {
    shape.constraints.iter().find_map(|c| match c {
        Constraint::MinLength(m) => Some(m.0),
        _ => None,
    })
}

fn constraint_max_length(shape: &Shape) -> Option<i32> {
    shape.constraints.iter().find_map(|c| match c {
        Constraint::MaxLength(m) => Some(m.0),
        _ => None,
    })
}

fn constraint_min_inclusive<'a>(shape: &Shape<'a>) -> Option<TermRef<'a>> {
    shape.constraints.iter().find_map(|c| match c {
        Constraint::MinInclusive(m) => Some(m.0),
        _ => None,
    })
}

fn constraint_max_inclusive<'a>(shape: &Shape<'a>) -> Option<TermRef<'a>> {
    shape.constraints.iter().find_map(|c| match c {
        Constraint::MaxInclusive(m) => Some(m.0),
        _ => None,
    })
}

/// L0006 (Warning): unsatisfiable bound pairs on the same shape -
/// minCount>maxCount, minLength>maxLength, or minInclusive>maxInclusive.
fn lint_l0006_unsatisfiable_bounds<'a>(
    graph: &'a Graph,
    shapes: &'a [Shape<'a>],
    diags: &mut Vec<Diagnostic>,
) {
    for shape in walk_shapes(shapes) {
        if let (Some(min), Some(max)) = (constraint_min_count(shape), constraint_max_count(shape)) {
            if min > max {
                push_diag(
                    diags,
                    graph,
                    "L0006",
                    DiagnosticSeverity::Warning,
                    shape.node,
                    Some("minCount"),
                    Some(format!("sh:minCount {} > sh:maxCount {}", min, max)),
                );
            }
        }
        if let (Some(min), Some(max)) = (constraint_min_length(shape), constraint_max_length(shape))
        {
            if min > max {
                push_diag(
                    diags,
                    graph,
                    "L0006",
                    DiagnosticSeverity::Warning,
                    shape.node,
                    Some("minLength"),
                    Some(format!("sh:minLength {} > sh:maxLength {}", min, max)),
                );
            }
        }
        if let (Some(min), Some(max)) = (
            constraint_min_inclusive(shape),
            constraint_max_inclusive(shape),
        ) {
            if let (Some(min_c), Some(max_c)) = (to_comparable(min), to_comparable(max)) {
                use crate::utils::compare_comparables;
                if compare_comparables(&min_c, &max_c, |ord| ord > 0) {
                    push_diag(
                        diags,
                        graph,
                        "L0006",
                        DiagnosticSeverity::Warning,
                        shape.node,
                        Some("minInclusive"),
                        Some(format!("sh:minInclusive {} > sh:maxInclusive {}", min, max)),
                    );
                }
            }
        }
    }
}

/// L0007 (Warning): `sh:class`/`sh:datatype` objects that are not IRIs, and
/// `sh:nodeKind` objects that are not one of the six `sh:` node-kind IRIs.
fn lint_l0007_wrong_term_kind(graph: &Graph, diags: &mut Vec<Diagnostic>) {
    for (predicate, keyword) in [(sh::CLASS, "class"), (sh::DATATYPE, "datatype")] {
        for triple in graph.triples_for_predicate(predicate) {
            if matches!(triple.object, TermRef::NamedNode(_)) {
                continue;
            }
            push_diag(
                diags,
                graph,
                "L0007",
                DiagnosticSeverity::Warning,
                triple.subject,
                Some(keyword),
                Some(triple.object.to_string()),
            );
        }
    }

    let valid_node_kinds = [
        sh::IRI,
        sh::BLANK_NODE,
        sh::LITERAL,
        sh::BLANK_NODE_OR_IRI,
        sh::BLANK_NODE_OR_LITERAL,
        sh::IRI_OR_LITERAL,
    ];
    for triple in graph.triples_for_predicate(sh::NODE_KIND_PROPERTY) {
        let is_valid =
            matches!(triple.object, TermRef::NamedNode(nn) if valid_node_kinds.contains(&nn));
        if !is_valid {
            push_diag(
                diags,
                graph,
                "L0007",
                DiagnosticSeverity::Warning,
                triple.subject,
                Some("nodeKind"),
                Some(triple.object.to_string()),
            );
        }
    }
}

/// L0008 (Warning): an empty `sh:in` or `sh:languageIn` list. Scans the raw
/// graph rather than parsed constraints, since the parser silently drops
/// `sh:in`/`sh:languageIn` entirely when their list is empty (see
/// `parser/constraints/sh_in.rs`), leaving nothing in `shape.constraints` to
/// inspect.
fn lint_l0008_empty_list(graph: &Graph, diags: &mut Vec<Diagnostic>) {
    for (predicate, keyword) in [(sh::IN, "in"), (sh::LANGUAGE_IN, "languageIn")] {
        for triple in graph.triples_for_predicate(predicate) {
            let Some(list_node) = term_to_named_or_blank(triple.object) else {
                continue;
            };
            if parse_rdf_list(graph, list_node).is_empty() {
                push_diag(
                    diags,
                    graph,
                    "L0008",
                    DiagnosticSeverity::Warning,
                    triple.subject,
                    Some(keyword),
                    None,
                );
            }
        }
    }
}

/// L0009 (Warning): a min/max-inclusive/exclusive bound that is not a
/// literal, or is a literal ill-formed for its own datatype - either way it
/// can never compare to anything.
fn lint_l0009_noncomparable_bound<'a>(
    graph: &'a Graph,
    shapes: &'a [Shape<'a>],
    diags: &mut Vec<Diagnostic>,
) {
    for shape in walk_shapes(shapes) {
        for constraint in &shape.constraints {
            let (term, keyword) = match constraint {
                Constraint::MinInclusive(m) => (m.0, "minInclusive"),
                Constraint::MinExclusive(m) => (m.0, "minExclusive"),
                Constraint::MaxInclusive(m) => (m.0, "maxInclusive"),
                Constraint::MaxExclusive(m) => (m.0, "maxExclusive"),
                _ => continue,
            };
            let non_comparable = match term {
                TermRef::Literal(lit) => literal_is_ill_formed(lit),
                _ => true,
            };
            if non_comparable {
                push_diag(
                    diags,
                    graph,
                    "L0009",
                    DiagnosticSeverity::Warning,
                    shape.node,
                    Some(keyword),
                    Some(term.to_string()),
                );
            }
        }
    }
}

/// L0010 (Warning): `sh:ignoredProperties` declared on a node without
/// `sh:closed true` alongside it, where it has no effect at all.
fn lint_l0010_ignored_without_closed(graph: &Graph, diags: &mut Vec<Diagnostic>) {
    for triple in graph.triples_for_predicate(sh::IGNORED_PROPERTIES) {
        let closed_true = matches!(
            graph.object_for_subject_predicate(triple.subject, sh::CLOSED),
            Some(TermRef::Literal(lit)) if lit.value() == "true"
        );
        if !closed_true {
            push_diag(
                diags,
                graph,
                "L0010",
                DiagnosticSeverity::Warning,
                triple.subject,
                Some("ignoredProperties"),
                None,
            );
        }
    }
}

/// True if `node` appears as the object of `sh:node`/`sh:property`/`sh:not`/
/// `sh:qualifiedValueShape` anywhere in `graph`, or as a member (including
/// nested) of an `rdf:List` that is the object of `sh:and`/`sh:or`/`sh:xone`.
fn is_referenced<'a>(graph: &'a Graph, node: NamedOrBlankNodeRef<'a>) -> bool {
    let direct = [sh::NODE, sh::PROPERTY, sh::NOT, sh::QUALIFIED_VALUE_SHAPE];
    for triple in graph.triples_for_object(TermRef::from(node)) {
        if direct.contains(&triple.predicate) {
            return true;
        }
    }
    for list_predicate in [sh::AND, sh::OR, sh::XONE] {
        for triple in graph.triples_for_predicate(list_predicate) {
            let Some(head) = term_to_named_or_blank(triple.object) else {
                continue;
            };
            if parse_rdf_list(graph, head)
                .iter()
                .any(|term| term_to_named_or_blank(*term) == Some(node))
            {
                return true;
            }
        }
    }
    false
}

/// L0011 (Info): a top-level parsed shape with no targets that is also
/// unreachable from every other shape - it is never evaluated.
fn lint_l0011_dead_shape<'a>(
    graph: &'a Graph,
    shapes: &'a [Shape<'a>],
    diags: &mut Vec<Diagnostic>,
) {
    for shape in shapes {
        if !shape.targets.is_empty() {
            continue;
        }
        if is_referenced(graph, shape.node) {
            continue;
        }
        push_diag(
            diags,
            graph,
            "L0011",
            DiagnosticSeverity::Info,
            shape.node,
            None,
            None,
        );
    }
}

/// L0012 (Info): `sh:deactivated true` - a reminder, not a defect.
fn lint_l0012_deactivated<'a>(
    graph: &'a Graph,
    shapes: &'a [Shape<'a>],
    diags: &mut Vec<Diagnostic>,
) {
    for shape in walk_shapes(shapes) {
        if shape.deactivated {
            push_diag(
                diags,
                graph,
                "L0012",
                DiagnosticSeverity::Info,
                shape.node,
                Some("deactivated"),
                None,
            );
        }
    }
}
