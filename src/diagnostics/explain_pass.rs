//! Traces *why* a single focus node does or does not conform to the shapes
//! graph: for each shape (optionally restricted to one via `shape_filter`),
//! reports whether/how it targets the focus node, then walks every
//! constraint - on the shape itself and on each of its property shapes -
//! reporting whether that constraint conforms, violates, or vacuously
//! conforms for this focus node. Unlike [`super::from_report`], this is not
//! a post-processing pass over a `ValidationReport`: it runs its own
//! single-focus-node validation per traced shape via
//! `Shape::validate_focus_node`.

use std::collections::HashMap;

use oxigraph::model::{NamedOrBlankNodeRef, TermRef};

use crate::decompose::shape_id_index;
use crate::validation::dataset::ValidationDataset;
use crate::validation::report::ValidationReport;
use crate::{Constraint, Shape, ValidationResult};

use super::derive::{build_shapes_snippet, stable_shape_display};
use super::registry;
use super::{sort_diagnostics, Diagnostic, DiagnosticSeverity, Verdict};

/// Explains conformance of `focus` against `shapes`, restricted to the
/// single shape named by `shape_filter` when given. Produces one
/// [`Diagnostic`] per shape that is targeted-but-unmatched (`NotTargeted`),
/// plus one per (shape-or-property-shape, constraint) pair for every shape
/// that *is* traced - all at [`DiagnosticSeverity::Info`], sorted
/// deterministically before returning.
pub fn explain_conformance<'a>(
    dataset: &'a ValidationDataset,
    shapes: &'a [Shape<'a>],
    focus: TermRef<'a>,
    shape_filter: Option<NamedOrBlankNodeRef<'a>>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let shape_ids = shape_id_index(shapes);

    for shape in shapes {
        if let Some(filter) = shape_filter {
            if shape.node != filter {
                continue;
            }
        }

        if shape.targets.is_empty() {
            if shape_filter.is_none() {
                // Never targeted, never referenced directly - nothing to
                // trace unless the caller asked for this shape by name.
                continue;
            }
            trace_shape(
                dataset,
                shape,
                focus,
                vec!["shape has no targets; evaluated directly".to_string()],
                &shape_ids,
                &mut diagnostics,
            );
            continue;
        }

        let mut selected_by = Vec::new();
        for &target in &shape.targets {
            let resolved = crate::validation::resolve_target(dataset, target);
            if resolved.contains(&focus) {
                selected_by.push(format!("selected by {}", target));
            }
        }

        if selected_by.is_empty() {
            let notes: Vec<String> = shape
                .targets
                .iter()
                .map(|target| format!("target: {}", target))
                .collect();
            diagnostics.push(Diagnostic {
                code: "V0000",
                severity: DiagnosticSeverity::Info,
                title: format!("focus node is not targeted by {}", shape.node),
                constraint_component: None,
                snippets: Vec::new(),
                expected: None,
                actual: None,
                notes,
                help: Some(
                    "add the focus node to a target, e.g. give it rdf:type matching sh:targetClass"
                        .to_string(),
                ),
                focus_node: Some(focus.to_string()),
                source_shape: Some(stable_shape_display(shape.node, &shape_ids)),
                path: None,
                verdict: Some(Verdict::NotTargeted),
            });
            continue;
        }

        trace_shape(
            dataset,
            shape,
            focus,
            selected_by,
            &shape_ids,
            &mut diagnostics,
        );
    }

    sort_diagnostics(&mut diagnostics);
    diagnostics
}

/// Runs a single-focus-node validation of `shape` (which reaches its
/// property shapes too, via `validate_focus_node`'s own recursion), then
/// emits one constraint trace per constraint on `shape` and per constraint
/// on each of its property shapes.
#[allow(clippy::too_many_arguments)]
fn trace_shape<'a>(
    dataset: &'a ValidationDataset,
    shape: &'a Shape<'a>,
    focus: TermRef<'a>,
    targeting_notes: Vec<String>,
    shape_ids: &HashMap<NamedOrBlankNodeRef<'a>, String>,
    out: &mut Vec<Diagnostic>,
) {
    out.push(shape_header_diagnostic(
        shape,
        focus,
        &targeting_notes,
        shape_ids,
    ));

    let mut report = ValidationReport::new();
    shape.validate_focus_node(dataset, focus, &mut report);

    let value_nodes = shape.get_value_nodes(dataset, focus);
    for constraint in &shape.constraints {
        out.push(constraint_diagnostic(
            dataset,
            shape,
            constraint,
            &value_nodes,
            focus,
            &report,
            &targeting_notes,
            shape_ids,
        ));
    }

    for property_shape in &shape.property_shapes {
        let value_nodes = property_shape.get_value_nodes(dataset, focus);
        for constraint in &property_shape.constraints {
            out.push(constraint_diagnostic(
                dataset,
                property_shape,
                constraint,
                &value_nodes,
                focus,
                &report,
                &targeting_notes,
                shape_ids,
            ));
        }
    }
}

/// A lightweight header entry identifying the shape being traced, so a
/// NodeShape whose constraints all live on nested (often anonymous
/// blank-node) property shapes still has a visible anchor: without this,
/// the only cards in its trace are its property shapes' own diagnostics,
/// whose `source_shape` is the property shape's own (frequently anonymous)
/// node, not the parent NodeShape's identity. Carries no verdict - it is
/// not itself a pass/fail statement, just identification.
fn shape_header_diagnostic<'a>(
    shape: &'a Shape<'a>,
    focus: TermRef<'a>,
    targeting_notes: &[String],
    shape_ids: &HashMap<NamedOrBlankNodeRef<'a>, String>,
) -> Diagnostic {
    let kind = if shape.is_property_shape() {
        "PropertyShape"
    } else {
        "NodeShape"
    };
    let title = match &shape.name {
        Some(name) => format!("{} {} ({})", kind, shape.node, name),
        None => format!("{} {}", kind, shape.node),
    };
    Diagnostic {
        code: "V0000",
        severity: DiagnosticSeverity::Info,
        title,
        constraint_component: None,
        snippets: Vec::new(),
        expected: None,
        actual: None,
        notes: targeting_notes.to_vec(),
        help: None,
        focus_node: Some(focus.to_string()),
        source_shape: Some(stable_shape_display(shape.node, shape_ids)),
        path: None,
        verdict: None,
    }
}

/// Traces one constraint on `constraint_owner` (either the traced shape
/// itself, for node-shape constraints, or one of its property shapes):
/// vacuous when `value_nodes` is empty (and the constraint isn't
/// `sh:minCount`, which is defined precisely for that case), violates when
/// the focus-node report carries a matching result, conforms otherwise.
#[allow(clippy::too_many_arguments)]
fn constraint_diagnostic<'a>(
    dataset: &'a ValidationDataset,
    constraint_owner: &'a Shape<'a>,
    constraint: &'a Constraint<'a>,
    value_nodes: &[TermRef<'a>],
    focus: TermRef<'a>,
    report: &ValidationReport<'a>,
    targeting_notes: &[String],
    shape_ids: &HashMap<NamedOrBlankNodeRef<'a>, String>,
) -> Diagnostic {
    let component_iri = constraint_component_iri(constraint);
    let code = registry::code_for_component(component_iri);

    let full_title = constraint.to_string();
    let title_line = full_title.lines().next().unwrap_or(&full_title);

    let mut notes: Vec<String> = targeting_notes.to_vec();

    let (verdict, title, help) = if value_nodes.is_empty()
        && !matches!(constraint, Constraint::MinCount(_))
    {
        let path = constraint_owner
            .path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default();
        notes.push(format!("path {path} resolved to 0 value nodes"));
        (
            Verdict::Vacuous,
            format!("{title_line} vacuously conforms"),
            Some(format!(
                "add sh:minCount 1 to {} if values are required",
                constraint_owner.node
            )),
        )
    } else if let Some(matching) = find_matching_result(report, constraint_owner.node, constraint) {
        notes.push(
            matching
                .messages()
                .first()
                .cloned()
                .unwrap_or_else(|| "constraint violated".to_string()),
        );
        (Verdict::Violates, format!("{title_line} is violated"), None)
    } else {
        notes.push(value_node_listing(value_nodes));
        (Verdict::Conforms, format!("{title_line} conforms"), None)
    };

    let (shapes_snippet, _bound) =
        build_shapes_snippet(dataset.shapes_graph(), constraint_owner.node, None);
    let snippets = shapes_snippet.into_iter().collect();

    Diagnostic {
        code,
        severity: DiagnosticSeverity::Info,
        title,
        constraint_component: Some(component_iri.to_string()),
        snippets,
        expected: None,
        actual: None,
        notes,
        help,
        focus_node: Some(focus.to_string()),
        source_shape: Some(stable_shape_display(constraint_owner.node, shape_ids)),
        path: constraint_owner.path.as_ref().map(|p| p.to_string()),
        verdict: Some(verdict),
    }
}

/// Up to 5 value nodes, then `"… and N more"`.
fn value_node_listing(value_nodes: &[TermRef<'_>]) -> String {
    if value_nodes.is_empty() {
        return "value nodes checked: (none)".to_string();
    }
    let mut parts: Vec<String> = value_nodes.iter().take(5).map(|v| v.to_string()).collect();
    if value_nodes.len() > 5 {
        parts.push(format!("… and {} more", value_nodes.len() - 5));
    }
    format!("value nodes checked: {}", parts.join(", "))
}

/// The first report result owned by `owner_node` whose component maps onto
/// the same registry code as `constraint`'s own component - i.e. the
/// violation, if any, that this constraint trace corresponds to.
fn find_matching_result<'a, 'b>(
    report: &'b ValidationReport<'a>,
    owner_node: NamedOrBlankNodeRef<'a>,
    constraint: &Constraint<'a>,
) -> Option<&'b ValidationResult<'a>> {
    report.get_results().iter().find(|result| {
        result.source_shape() == owner_node && matches_constraint(result, constraint)
    })
}

/// `sh:qualifiedValueShape` can surface as either the min- or max-count
/// component depending on which bound tripped; `sh:sparql` results carry
/// either `SPARQLConstraintComponent` or a query-supplied custom component
/// (which falls back to the registry's `V0000`). Every other constraint maps
/// onto exactly one component/code.
///
/// A shared component code alone isn't always enough: a shape can carry
/// several constraints of the *same* component on the same path - e.g.
/// `sh:class ex:Person, ex:Employee` parses into two separate
/// `Constraint::Class` instances, one per value (see
/// `parser/constraints/class.rs`, which calls `objects_for_subject_predicate`
/// rather than the singular `object_for_subject_predicate`). Matching by
/// component code alone would attribute *both* traces to whichever one of
/// the two results happens to come first, even the one that conforms. So
/// once the component matches, `constraint_detail` further requires the
/// constraint's own identifying detail (when it has one) to match the
/// detail the validator recorded on the result.
fn matches_constraint(result: &ValidationResult<'_>, constraint: &Constraint<'_>) -> bool {
    let code = match result.source_constraint_component() {
        Some(iri) => registry::code_for_component(iri.as_str()),
        None => "V0000",
    };
    let component_matches = match constraint {
        Constraint::QualifiedValueShape(_) => code == "V0022" || code == "V0023",
        Constraint::Sparql(_) => code == "V0029" || code == "V0000",
        other => code == registry::code_for_component(constraint_component_iri(other)),
    };
    component_matches
        && match constraint_detail(constraint) {
            // The constraint has an identifying detail (formatted exactly as
            // the validator's `.detail(...)` call for that component - see
            // `validation/constraints/{class,has_value,datatype,node_kind}.rs`).
            // Only a result recorded with that same detail belongs to this
            // constraint instance. A result with no detail can't be matched
            // this way - conservatively excluded rather than assumed to
            // match, since only components that only ever appear once per
            // shape (so misattribution can't happen) skip recording one.
            Some(detail) => result.constraint_detail() == Some(detail.as_str()),
            None => true,
        }
}

/// The identifying detail string for constraint variants whose parser can
/// produce more than one instance per shape/path (`Constraint::Class` and
/// `Constraint::HasValue`; see `parser/constraints/class.rs` and
/// `parser/constraints/has_value.rs`, both of which iterate
/// `objects_for_subject_predicate` rather than taking a single object).
/// `Constraint::Datatype` and `Constraint::NodeKind` also record a detail on
/// their results but their parsers take only a single object per shape
/// (`object_for_subject_predicate`), so they can never actually duplicate;
/// their detail is still checked here for robustness, at no cost, since it's
/// always unique per shape when present. Mirrors the `.detail(format!(...))`
/// calls in the corresponding `validation/constraints/*.rs` files - keep the
/// two in sync.
fn constraint_detail(constraint: &Constraint<'_>) -> Option<String> {
    match constraint {
        Constraint::Class(c) => Some(format!("sh:class {}", c.0)),
        Constraint::HasValue(c) => Some(format!("sh:hasValue {}", c.0)),
        Constraint::Datatype(c) => Some(format!("sh:datatype {}", c.0)),
        Constraint::NodeKind(c) => Some(format!("sh:nodeKind {}", c.0)),
        _ => None,
    }
}

/// Maps a `Constraint` variant onto the `sh:*ConstraintComponent` IRI the
/// validator attaches to the `ValidationResult`s it produces for that
/// variant (see `registry::code_for_component`'s table).
fn constraint_component_iri(constraint: &Constraint<'_>) -> &'static str {
    match constraint {
        Constraint::Class(_) => "http://www.w3.org/ns/shacl#ClassConstraintComponent",
        Constraint::Datatype(_) => "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
        Constraint::NodeKind(_) => "http://www.w3.org/ns/shacl#NodeKindConstraintComponent",
        Constraint::MinCount(_) => "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
        Constraint::MaxCount(_) => "http://www.w3.org/ns/shacl#MaxCountConstraintComponent",
        Constraint::MinExclusive(_) => "http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent",
        Constraint::MinInclusive(_) => "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent",
        Constraint::MaxExclusive(_) => "http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent",
        Constraint::MaxInclusive(_) => "http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent",
        Constraint::MinLength(_) => "http://www.w3.org/ns/shacl#MinLengthConstraintComponent",
        Constraint::MaxLength(_) => "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent",
        Constraint::Pattern(_) => "http://www.w3.org/ns/shacl#PatternConstraintComponent",
        Constraint::LanguageIn(_) => "http://www.w3.org/ns/shacl#LanguageInConstraintComponent",
        Constraint::UniqueLang(_) => "http://www.w3.org/ns/shacl#UniqueLangConstraintComponent",
        Constraint::Equals(_) => "http://www.w3.org/ns/shacl#EqualsConstraintComponent",
        Constraint::Disjoint(_) => "http://www.w3.org/ns/shacl#DisjointConstraintComponent",
        Constraint::LessThan(_) => "http://www.w3.org/ns/shacl#LessThanConstraintComponent",
        Constraint::LessThanOrEquals(_) => {
            "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent"
        }
        Constraint::HasValue(_) => "http://www.w3.org/ns/shacl#HasValueConstraintComponent",
        Constraint::In(_) => "http://www.w3.org/ns/shacl#InConstraintComponent",
        Constraint::Node(_) => "http://www.w3.org/ns/shacl#NodeConstraintComponent",
        Constraint::QualifiedValueShape(_) => {
            "http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent"
        }
        Constraint::And(_) => "http://www.w3.org/ns/shacl#AndConstraintComponent",
        Constraint::Or(_) => "http://www.w3.org/ns/shacl#OrConstraintComponent",
        Constraint::Xone(_) => "http://www.w3.org/ns/shacl#XoneConstraintComponent",
        Constraint::Not(_) => "http://www.w3.org/ns/shacl#NotConstraintComponent",
        Constraint::Sparql(_) => "http://www.w3.org/ns/shacl#SPARQLConstraintComponent",
    }
}

/// Every shape with at least one target, paired with the Display strings of
/// its distinct resolved focus nodes - conforming and violating alike. This
/// is the data behind the web demo's "Shapes & Focus Nodes" browser, which
/// lets a user click a *conforming* node and ask "why did this pass?"
/// instead of only ever landing on nodes that already appear in a
/// violation. Shapes with no targets are omitted (nothing to browse).
pub fn shape_target_nodes<'a>(
    dataset: &'a ValidationDataset,
    shapes: &'a [Shape<'a>],
) -> Vec<(String, Vec<String>)> {
    shapes
        .iter()
        .filter(|shape| !shape.targets.is_empty())
        .map(|shape| {
            let nodes: std::collections::BTreeSet<String> = shape
                .targets
                .iter()
                .flat_map(|&target| crate::validation::resolve_target(dataset, target))
                .map(|term| term.to_string())
                .collect();
            (shape.node.to_string(), nodes.into_iter().collect())
        })
        .collect()
}
