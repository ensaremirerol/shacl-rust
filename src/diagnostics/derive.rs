//! Derives rich [`Diagnostic`]s from a [`ValidationReport`]: for each
//! validation result, reconstructs the offending data triple and the
//! declaring shape triples as annotated Turtle snippets, maps the
//! `sh:sourceConstraintComponent` onto a stable registry code, and fills in
//! an expected/actual/help summary from a small per-component table.

use oxigraph::model::{NamedOrBlankNodeRef, TermRef};

use crate::core::path::PathElement;
use crate::validation::dataset::ValidationDataset;
use crate::{Path, Shape, ValidationReport, ValidationResult};

use super::registry;
use super::{Diagnostic, DiagnosticSeverity, Snippet, SnippetOrigin};

/// Derives one [`Diagnostic`] per validation result, sorted deterministically
/// via [`super::sort_diagnostics`].
pub fn from_report<'a>(
    report: &ValidationReport<'a>,
    dataset: &'a ValidationDataset,
    shapes: &'a [Shape<'a>],
) -> Vec<Diagnostic> {
    let mut diags: Vec<Diagnostic> = report
        .get_results()
        .iter()
        .map(|result| derive_one(result, dataset, shapes))
        .collect();
    super::sort_diagnostics(&mut diags);
    diags
}

fn derive_one<'a>(
    result: &ValidationResult<'a>,
    dataset: &'a ValidationDataset,
    shapes: &'a [Shape<'a>],
) -> Diagnostic {
    // Rule 1: code from the constraint component, V0000 fallback either way.
    let component_iri = result
        .source_constraint_component()
        .map(|c| c.as_str().to_string());
    let code = match &component_iri {
        Some(iri) => registry::code_for_component(iri),
        None => "V0000",
    };
    let reg_entry = registry::entry(code);

    // Rule 2: title from the registry, falling back to the first message.
    let title = reg_entry
        .map(|e| e.title.to_string())
        .unwrap_or_else(|| result.messages().first().cloned().unwrap_or_default());

    // Rule 3: severity from the result's severity IRI.
    let severity = severity_from_iri(result.severity().as_str());

    let path_display = result.result_path().map(|p| p.to_string());
    let keyword = component_iri.as_deref().map(component_keyword);

    // Rule 5: shapes snippet, plus the constraint-object "bound" lookup that
    // rule 6 needs (e.g. the object of the sh:minInclusive triple).
    let (shapes_snippet, bound) =
        build_shapes_snippet(dataset, result.source_shape(), keyword.as_deref());

    // Rule 6: expected/actual/help table, keyed by component keyword.
    let table = component_table(
        keyword.as_deref(),
        bound.as_deref(),
        path_display.as_deref(),
        &title,
        reg_entry,
    );

    // Rule 4: data snippet (skipped entirely when there is no value).
    let mut snippets = Vec::new();
    if let Some(value) = result.value() {
        snippets.push(build_data_snippet(
            result.focus_node(),
            result.result_path(),
            value,
            &table.annotation,
        ));
    }
    if let Some(shapes_snippet) = shapes_snippet {
        snippets.push(shapes_snippet);
    }

    // Rule 7: actual, unless the component table overrides it (cardinality
    // constraints have no single "value" - the count lives in the message).
    let actual = if table.actual_none {
        None
    } else {
        result.value().map(|v| v.to_string())
    };

    // Rule 8: raw messages, plus a note naming the target that selected the
    // focus node, when the owning top-level shape can be found.
    let mut notes: Vec<String> = result.messages().to_vec();
    if let Some(owner) = find_owning_shape(shapes, result.source_shape()) {
        for target in &owner.targets {
            notes.push(format!("focus node selected by {}", target));
        }
    }

    Diagnostic {
        code,
        severity,
        title,
        constraint_component: component_iri,
        snippets,
        expected: table.expected,
        actual,
        notes,
        help: table.help,
        focus_node: Some(result.focus_node().to_string()),
        source_shape: Some(result.source_shape().to_string()),
        path: path_display,
        verdict: None,
    }
}

fn severity_from_iri(iri: &str) -> DiagnosticSeverity {
    if iri.ends_with("#Violation") {
        DiagnosticSeverity::Error
    } else if iri.ends_with("#Warning") {
        DiagnosticSeverity::Warning
    } else {
        DiagnosticSeverity::Info
    }
}

/// `MinInclusiveConstraintComponent` -> `minInclusive`; `SPARQLConstraintComponent`
/// is special-cased to `sparql` since lowercasing only its first letter would
/// otherwise yield the meaningless `sPARQL`.
fn component_keyword(iri: &str) -> String {
    let local = iri.rsplit(['#', '/']).next().unwrap_or(iri);
    let stripped = local.strip_suffix("ConstraintComponent").unwrap_or(local);
    if stripped.eq_ignore_ascii_case("sparql") {
        return "sparql".to_string();
    }
    let mut chars = stripped.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The lexical form of a literal term, or its plain `Display` string for
/// anything else (IRIs, blank nodes).
fn plain_string(term: TermRef<'_>) -> String {
    match term {
        TermRef::Literal(lit) => lit.value().to_string(),
        other => other.to_string(),
    }
}

/// Rule 4: the reconstructed data-graph triple (or bare focus node for node
/// shapes), with `highlight` set to the offending value.
fn build_data_snippet<'a>(
    focus: TermRef<'a>,
    path: Option<&Path<'a>>,
    value: TermRef<'a>,
    annotation: &str,
) -> Snippet {
    let turtle = match path {
        Some(path) => format!("{} {} {} .", focus, first_path_predicate(path), value),
        None => format!("{} .", focus),
    };
    Snippet {
        origin: SnippetOrigin::DataGraph,
        turtle,
        highlight: value.to_string(),
        annotation: annotation.to_string(),
    }
}

/// The predicate slot for rule 4's data snippet: the first path element when
/// it is a plain IRI step, otherwise the path's own `Display` (covers
/// inverse/sequence/alternative paths that have no single leading predicate).
fn first_path_predicate<'a>(path: &Path<'a>) -> String {
    match path.get_elements().first() {
        Some(PathElement::Iri(predicate)) => predicate.to_string(),
        _ => path.to_string(),
    }
}

/// Rule 5: up to 8 triples of the source shape node, rendered as Turtle, plus
/// the object of whichever triple's predicate matches the component keyword
/// via [`select_predicate_index`] (rule 6's "bound" lookup).
fn build_shapes_snippet<'a>(
    dataset: &'a ValidationDataset,
    source_shape: NamedOrBlankNodeRef<'a>,
    keyword: Option<&str>,
) -> (Option<Snippet>, Option<String>) {
    let triples: Vec<_> = dataset
        .shapes_graph()
        .triples_for_subject(source_shape)
        .take(8)
        .collect();
    if triples.is_empty() {
        return (None, None);
    }

    let last = triples.len() - 1;
    let lines: Vec<String> = triples
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let terminator = if i == last { "." } else { ";" };
            if i == 0 {
                format!(
                    "{} {} {} {}",
                    source_shape, t.predicate, t.object, terminator
                )
            } else {
                format!("    {} {} {}", t.predicate, t.object, terminator)
            }
        })
        .collect();
    let turtle = lines.join("\n");

    let predicate_iris: Vec<String> = triples
        .iter()
        .map(|t| t.predicate.as_str().to_string())
        .collect();
    let matched_index = keyword.and_then(|kw| select_predicate_index(&predicate_iris, kw));
    let bound = matched_index.map(|i| plain_string(triples[i].object));
    let highlight = matched_index
        .map(|i| lines[i].trim_start().to_string())
        .unwrap_or_else(|| lines[0].trim_start().to_string());

    let snippet = Snippet {
        origin: SnippetOrigin::ShapesGraph,
        turtle,
        highlight,
        annotation: "constraint declared here".to_string(),
    };
    (Some(snippet), bound)
}

/// Selects the index of the predicate IRI in `predicate_iris` that
/// corresponds to `keyword` (a [`component_keyword`] value, e.g. `"in"` or
/// `"disjoint"`). Matches by exact predicate local name (the IRI fragment
/// after `#`, or the final path segment after `/`) first, so that e.g. an
/// `sh:in` constraint is never shadowed by an unrelated `sh:disjoint`
/// triple merely because `"in"` is a substring of `"disjoint"`. Falls back
/// to a raw substring `contains` only when no predicate's local name
/// matches exactly.
fn select_predicate_index(predicate_iris: &[String], keyword: &str) -> Option<usize> {
    predicate_iris
        .iter()
        .position(|iri| local_name(iri) == keyword)
        .or_else(|| predicate_iris.iter().position(|iri| iri.contains(keyword)))
}

/// The fragment/local-name portion of an IRI: the substring after the last
/// `#`, or after the last `/` if there is no `#`, or the whole string if
/// neither is present.
fn local_name(iri: &str) -> &str {
    iri.rsplit(['#', '/']).next().unwrap_or(iri)
}

/// The result of rule 6's per-component lookup table.
struct ComponentTable {
    annotation: String,
    expected: Option<String>,
    help: Option<String>,
    actual_none: bool,
}

/// The first sentence of `text`, trailing period included: scans for the
/// first ". " boundary that isn't immediately preceded by the abbreviations
/// `"e.g"`, `"i.e"`, or `"etc"` (checked as the 3 characters before the
/// period), so that explanations like `"...(e.g. en matches en-NZ). Second
/// sentence."` truncate after the real sentence rather than after `"(e.g"`.
/// Falls back to the whole text when no such boundary is found.
fn first_sentence(text: &str) -> &str {
    let mut search_start = 0usize;
    while let Some(rel) = text[search_start..].find(". ") {
        let period_idx = search_start + rel;
        let preceding = if period_idx >= 3 {
            &text[period_idx - 3..period_idx]
        } else {
            ""
        };
        if matches!(preceding, "e.g" | "i.e" | "etc") {
            search_start = period_idx + 2;
            continue;
        }
        return &text[..=period_idx];
    }
    text
}

fn component_table(
    keyword: Option<&str>,
    bound: Option<&str>,
    path: Option<&str>,
    title: &str,
    reg_entry: Option<&registry::RegistryEntry>,
) -> ComponentTable {
    let help = reg_entry.map(|e| first_sentence(e.explanation).to_string());
    let path = path.unwrap_or("");

    let (annotation, expected, actual_none) = match keyword {
        Some("minInclusive") => (
            "this value is less than the required minimum",
            bound.map(|b| format!("a literal >= {}", b)),
            false,
        ),
        Some("minExclusive") => (
            "this value is not greater than the exclusive minimum",
            bound.map(|b| format!("a literal > {}", b)),
            false,
        ),
        Some("maxInclusive") => (
            "this value is greater than the required maximum",
            bound.map(|b| format!("a literal <= {}", b)),
            false,
        ),
        Some("maxExclusive") => (
            "this value is not less than the exclusive maximum",
            bound.map(|b| format!("a literal < {}", b)),
            false,
        ),
        Some("minCount") => {
            let n = bound.unwrap_or("");
            return ComponentTable {
                annotation: title.to_string(),
                expected: Some(format!("at least {} value(s) for path {}", n, path)),
                help,
                actual_none: true,
            };
        }
        Some("maxCount") => {
            let n = bound.unwrap_or("");
            return ComponentTable {
                annotation: title.to_string(),
                expected: Some(format!("at most {} value(s) for path {}", n, path)),
                help,
                actual_none: true,
            };
        }
        Some("datatype") => (
            title,
            bound.map(|b| format!("a literal with datatype {}", b)),
            false,
        ),
        Some("class") => (title, bound.map(|b| format!("an instance of {}", b)), false),
        Some("pattern") => (
            title,
            bound.map(|b| format!("a value matching {}", b)),
            false,
        ),
        Some("in") => (title, Some("one of the sh:in list".to_string()), false),
        Some("closed") => (
            title,
            Some(format!(
                "only declared properties; property {} is not allowed",
                path
            )),
            false,
        ),
        _ => (title, None, false),
    };

    ComponentTable {
        annotation: annotation.to_string(),
        expected,
        help,
        actual_none,
    }
}

/// Rule 8: the top-level shape that "owns" `source_shape` - either it *is*
/// that shape, or one of its (one-level-nested) property shapes is.
fn find_owning_shape<'a, 'b>(
    shapes: &'b [Shape<'a>],
    source_shape: NamedOrBlankNodeRef<'a>,
) -> Option<&'b Shape<'a>> {
    shapes.iter().find(|s| {
        s.node == source_shape || s.property_shapes.iter().any(|p| p.node == source_shape)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sentence_skips_abbreviations() {
        assert_eq!(
            first_sentence("Matches BCP 47 ranges (e.g. en matches en-NZ). Second sentence."),
            "Matches BCP 47 ranges (e.g. en matches en-NZ)."
        );
        assert_eq!(first_sentence("One sentence only"), "One sentence only");
        assert_eq!(first_sentence("Plain. Tail."), "Plain.");
        assert_eq!(
            first_sentence("That is, i.e. this. Tail."),
            "That is, i.e. this."
        );
    }

    #[test]
    fn keyword_matches_exact_local_name_over_substring() {
        // simulate the selection logic on rendered predicate IRIs
        let lines = vec![
            "http://www.w3.org/ns/shacl#disjoint".to_string(),
            "http://www.w3.org/ns/shacl#in".to_string(),
        ];
        assert_eq!(select_predicate_index(&lines, "in"), Some(1));
    }
}
