//! Structured, content-stable decomposition of a parsed shapes graph: one
//! JSON entry per individual constraint parameter binding, with recursive
//! `children` for logical constraints, and deterministic IDs independent of
//! blank-node labels, prefixes, or unrelated graph edits.
//!
//! Written for the SHACL Manager capability requirements (2026-07-31),
//! requirements R-1 (structured decomposition) and R-2 (stable IDs). This
//! module implements exactly the ID algorithm documented there:
//!
//! ```text
//! shape_id          = sha256( "shape" || canonical(shape IRI) )              named shapes
//! property_shape_id = sha256( "property-shape" || owner_id || canonical(path) || canonical(param set) )
//! constraint_id      = sha256( "constraint" || owner_id || component IRI || canonical(parameter values) )
//! ```
//!
//! Two extensions beyond what's explicitly specified, both following the
//! spec's own stated principle ("nested anonymous shapes hashed recursively
//! by content, never by blank-node label"):
//! - A top-level shape with a blank-node identity (rare, but legal SHACL)
//!   uses `owner_id = "root"` in the `property_shape_id`-shaped formula
//!   above, since it has no IRI of its own to hash.
//! - A shape nested inside `sh:and`/`sh:or`/`sh:xone`/`sh:not`/`sh:node`/
//!   `sh:qualifiedValueShape` (not reached via `sh:property`) uses the same
//!   formula with `path = ""` and a role marker (`"and"`, `"or"`, ...)
//!   folded into the canonical param set, so it can never collide with an
//!   actual empty-path property shape.
//!
//! `sh:qualifiedValueShape` is a further simplification: the SHACL spec
//! treats `sh:qualifiedMinCount`/`sh:qualifiedMaxCount` as two independent
//! constraint components (`QualifiedMinCountConstraintComponent` /
//! `QualifiedMaxCountConstraintComponent`), but this crate's parsed model
//! bundles both into one [`crate::core::constraints::QualifiedValueShapeConstraint`].
//! Decomposition reports it as a single entry, using
//! `QualifiedMinCountConstraintComponent` when `sh:qualifiedMinCount` is
//! present, else `QualifiedMaxCountConstraintComponent`.

use sha2::{Digest, Sha256};

use oxigraph::model::{NamedOrBlankNodeRef, TermRef};
use serde_json::{json, Map, Value};

use crate::core::constraints::*;
use crate::core::shape::{ClosedConstraint, Shape};
use crate::core::target::Target;
use crate::vocab::sh;

/// Hashes `parts` with a null-byte separator between each (so `["a", "bc"]`
/// and `["ab", "c"]` never collide) and returns a short, prefixed hex digest.
fn hash_parts(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            hasher.update([0u8]);
        }
        hasher.update(part.as_bytes());
    }
    format!("{prefix}:{:x}", hasher.finalize())
}

/// Canonical, prefix-independent string form of a term for hashing and for
/// JSON `parameters` values: full IRI for named nodes, canonical XSD lexical
/// form + datatype for literals (via the same `oxsdatatypes` parsing used
/// elsewhere in this crate for numeric/date comparison - see
/// `crate::utils::to_comparable`), blank nodes render as `_:` (their content
/// identity, if they need one, comes from the caller hashing the *shape*
/// they represent, not this raw term form).
fn canonical_term(term: TermRef<'_>) -> String {
    match term {
        TermRef::NamedNode(n) => n.as_str().to_string(),
        TermRef::BlankNode(b) => format!("_:{}", b.as_str()),
        TermRef::Literal(lit) => {
            let dt = lit.datatype();
            let canonical_value = if dt == oxigraph::model::vocab::xsd::INTEGER {
                lit.value()
                    .parse::<oxsdatatypes::Integer>()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| lit.value().to_string())
            } else if dt == oxigraph::model::vocab::xsd::DECIMAL {
                lit.value()
                    .parse::<oxsdatatypes::Decimal>()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| lit.value().to_string())
            } else if dt == oxigraph::model::vocab::xsd::DOUBLE
                || dt == oxigraph::model::vocab::xsd::FLOAT
            {
                lit.value()
                    .parse::<oxsdatatypes::Double>()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| lit.value().to_string())
            } else if dt == oxigraph::model::vocab::xsd::BOOLEAN {
                lit.value()
                    .parse::<oxsdatatypes::Boolean>()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| lit.value().to_string())
            } else {
                lit.value().to_string()
            };
            match lit.language() {
                Some(lang) => format!("\"{}\"@{}", canonical_value, lang),
                None => format!("\"{}\"^^{}", canonical_value, dt.as_str()),
            }
        }
    }
}

/// JSON rendering of a term for `parameters` values: `{"value", "datatype"}`
/// for literals (matching R-1's worked example), the bare IRI string for
/// named nodes.
fn term_to_json(term: TermRef<'_>) -> Value {
    match term {
        TermRef::NamedNode(n) => json!(n.as_str()),
        TermRef::BlankNode(b) => json!(format!("_:{}", b.as_str())),
        TermRef::Literal(lit) => {
            let mut obj = Map::new();
            obj.insert("value".to_string(), json!(lit.value()));
            if let Some(lang) = lit.language() {
                obj.insert("language".to_string(), json!(lang));
            } else {
                obj.insert("datatype".to_string(), json!(lit.datatype().as_str()));
            }
            Value::Object(obj)
        }
    }
}

/// Canonical SPARQL property-path syntax for hashing and display. Full IRIs
/// (never CURIEs - stable across prefix renames, at the cost of verbosity);
/// this is `Path`'s existing `Display`, which already renders full IRIs.
fn canonical_path(path: &crate::core::path::Path<'_>) -> String {
    path.to_string()
}

/// The `sh:*ConstraintComponent` IRI for a parsed constraint, matching the
/// component every validator already reports on violations for it.
fn component_iri(constraint: &Constraint<'_>) -> &'static str {
    let nn = match constraint {
        Constraint::Class(_) => sh::CLASS_CONSTRAINT_COMPONENT,
        Constraint::Datatype(_) => sh::DATATYPE_CONSTRAINT_COMPONENT,
        Constraint::NodeKind(_) => sh::NODE_KIND_CONSTRAINT_COMPONENT,
        Constraint::MinCount(_) => sh::MIN_COUNT_CONSTRAINT_COMPONENT,
        Constraint::MaxCount(_) => sh::MAX_COUNT_CONSTRAINT_COMPONENT,
        Constraint::MinExclusive(_) => sh::MIN_EXCLUSIVE_CONSTRAINT_COMPONENT,
        Constraint::MinInclusive(_) => sh::MIN_INCLUSIVE_CONSTRAINT_COMPONENT,
        Constraint::MaxExclusive(_) => sh::MAX_EXCLUSIVE_CONSTRAINT_COMPONENT,
        Constraint::MaxInclusive(_) => sh::MAX_INCLUSIVE_CONSTRAINT_COMPONENT,
        Constraint::MinLength(_) => sh::MIN_LENGTH_CONSTRAINT_COMPONENT,
        Constraint::MaxLength(_) => sh::MAX_LENGTH_CONSTRAINT_COMPONENT,
        Constraint::Pattern(_) => sh::PATTERN_CONSTRAINT_COMPONENT,
        Constraint::LanguageIn(_) => sh::LANGUAGE_IN_CONSTRAINT_COMPONENT,
        Constraint::UniqueLang(_) => sh::UNIQUE_LANG_CONSTRAINT_COMPONENT,
        Constraint::Equals(_) => sh::EQUALS_CONSTRAINT_COMPONENT,
        Constraint::Disjoint(_) => sh::DISJOINT_CONSTRAINT_COMPONENT,
        Constraint::LessThan(_) => sh::LESS_THAN_CONSTRAINT_COMPONENT,
        Constraint::LessThanOrEquals(_) => sh::LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT,
        Constraint::HasValue(_) => sh::HAS_VALUE_CONSTRAINT_COMPONENT,
        Constraint::In(_) => sh::IN_CONSTRAINT_COMPONENT,
        Constraint::Node(_) => sh::NODE_CONSTRAINT_COMPONENT,
        Constraint::QualifiedValueShape(c) => {
            if c.qualified_min_count.is_some() {
                sh::QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT
            } else {
                sh::QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT
            }
        }
        Constraint::And(_) => sh::AND_CONSTRAINT_COMPONENT,
        Constraint::Or(_) => sh::OR_CONSTRAINT_COMPONENT,
        Constraint::Xone(_) => sh::XONE_CONSTRAINT_COMPONENT,
        Constraint::Not(_) => sh::NOT_CONSTRAINT_COMPONENT,
        Constraint::Sparql(_) => sh::SPARQL_CONSTRAINT_COMPONENT,
    };
    nn.as_str()
}

/// A constraint's own parameter set, both as JSON (`parameters`) and as a
/// canonical string (folded into its `constraint_id`). Constraints holding
/// nested shapes (`sh:and/or/xone/not/node/qualifiedValueShape`) get their
/// children decomposed first by the caller; `child_ids` carries the
/// already-computed, order-preserving IDs to fold in here instead of
/// re-deriving shape content from inside this function.
fn constraint_parameters(constraint: &Constraint<'_>, child_ids: &[String]) -> (Value, String) {
    match constraint {
        Constraint::Class(c) => {
            let v = c.0.as_str();
            (json!({"class": v}), v.to_string())
        }
        Constraint::Datatype(c) => {
            let v = c.0.as_str();
            (json!({"datatype": v}), v.to_string())
        }
        Constraint::NodeKind(c) => {
            let v = c.0.to_string();
            (json!({"nodeKind": v}), v)
        }
        Constraint::MinCount(c) => (json!({"minCount": c.0}), c.0.to_string()),
        Constraint::MaxCount(c) => (json!({"maxCount": c.0}), c.0.to_string()),
        Constraint::MinExclusive(c) => (
            json!({"minExclusive": term_to_json(c.0)}),
            canonical_term(c.0),
        ),
        Constraint::MinInclusive(c) => (
            json!({"minInclusive": term_to_json(c.0)}),
            canonical_term(c.0),
        ),
        Constraint::MaxExclusive(c) => (
            json!({"maxExclusive": term_to_json(c.0)}),
            canonical_term(c.0),
        ),
        Constraint::MaxInclusive(c) => (
            json!({"maxInclusive": term_to_json(c.0)}),
            canonical_term(c.0),
        ),
        Constraint::MinLength(c) => (json!({"minLength": c.0}), c.0.to_string()),
        Constraint::MaxLength(c) => (json!({"maxLength": c.0}), c.0.to_string()),
        Constraint::Pattern(c) => {
            let canon = format!("{}\u{0}{}", c.pattern, c.flags.as_deref().unwrap_or(""));
            (json!({"pattern": c.pattern, "flags": c.flags}), canon)
        }
        Constraint::LanguageIn(c) => (json!({"languageIn": c.0}), c.0.join("\u{0}")),
        Constraint::UniqueLang(c) => (json!({"uniqueLang": c.0}), c.0.to_string()),
        Constraint::Equals(c) => {
            let v = canonical_path(&c.0);
            (json!({"equals": v}), v)
        }
        Constraint::Disjoint(c) => {
            let v = canonical_path(&c.0);
            (json!({"disjoint": v}), v)
        }
        Constraint::LessThan(c) => {
            let v = canonical_path(&c.0);
            (json!({"lessThan": v}), v)
        }
        Constraint::LessThanOrEquals(c) => {
            let v = canonical_path(&c.0);
            (json!({"lessThanOrEquals": v}), v)
        }
        Constraint::HasValue(c) => (json!({"hasValue": term_to_json(c.0)}), canonical_term(c.0)),
        Constraint::In(c) => {
            let terms: Vec<Value> = c.0.iter().map(|t| term_to_json(*t)).collect();
            let canon =
                c.0.iter()
                    .map(|t| canonical_term(*t))
                    .collect::<Vec<_>>()
                    .join("\u{0}");
            (json!({"in": terms}), canon)
        }
        Constraint::Node(_) => (
            json!({"node": child_ids.first()}),
            child_ids.first().cloned().unwrap_or_default(),
        ),
        Constraint::QualifiedValueShape(c) => (
            json!({
                "qualifiedValueShape": child_ids.first(),
                "qualifiedMinCount": c.qualified_min_count,
                "qualifiedMaxCount": c.qualified_max_count,
                "qualifiedValueShapesDisjoint": c.qualified_value_shapes_disjoint,
            }),
            format!(
                "{}\u{0}{:?}\u{0}{:?}\u{0}{}",
                child_ids.first().cloned().unwrap_or_default(),
                c.qualified_min_count,
                c.qualified_max_count,
                c.qualified_value_shapes_disjoint
            ),
        ),
        Constraint::And(_) | Constraint::Or(_) | Constraint::Xone(_) => {
            (json!({ "shapes": child_ids }), child_ids.join("\u{0}"))
        }
        Constraint::Not(_) => (
            json!({"not": child_ids.first()}),
            child_ids.first().cloned().unwrap_or_default(),
        ),
        Constraint::Sparql(c) => {
            let (kind, query) = match &c.executable {
                SparqlExecutable::Select(q) => ("select", q.as_str()),
                SparqlExecutable::Ask(q) => ("ask", q.as_str()),
            };
            (
                json!({ kind: query, "message": c.messages }),
                format!("{kind}\u{0}{query}"),
            )
        }
    }
}

/// One entry in the flattened `constraints` array, or a `children` entry
/// under a logical constraint.
struct ConstraintEntry {
    id: String,
    component: &'static str,
    path: Option<String>,
    parameters: Value,
    owner_property_shape: Option<String>,
    severity: String,
    messages: Vec<String>,
    children: Option<Vec<ConstraintEntry>>,
}

impl ConstraintEntry {
    fn to_json(&self, source: Option<&str>) -> Value {
        let mut obj = Map::new();
        obj.insert("id".to_string(), json!(self.id));
        obj.insert("component".to_string(), json!(self.component));
        obj.insert("path".to_string(), json!(self.path));
        obj.insert("parameters".to_string(), self.parameters.clone());
        obj.insert(
            "owner_property_shape".to_string(),
            json!(self.owner_property_shape),
        );
        obj.insert("severity".to_string(), json!(self.severity));
        obj.insert("messages".to_string(), json!(self.messages));
        obj.insert("source".to_string(), json!(source));
        // R-5 (source spans): not implemented - oxigraph's Turtle parser
        // discards position information for successfully-parsed triples, so
        // there is currently no path from a parsed Shape back to its
        // original source line/column. Always null until that upstream gap
        // is closed.
        obj.insert("span".to_string(), Value::Null);
        if let Some(children) = &self.children {
            obj.insert(
                "children".to_string(),
                Value::Array(children.iter().map(|c| c.to_json(source)).collect()),
            );
        }
        Value::Object(obj)
    }
}

/// Recursively decomposes a shape reached via a logical constraint (or,
/// symmetrically, a property shape) into its own `shape_id` plus a flat list
/// of [`ConstraintEntry`] for its direct constraints (each further recursing
/// into its own nested shapes). `owner_id`/`role` feed the content hash;
/// `path` is `""` for logical-constraint members (they have no path of their
/// own) and the property shape's own canonical path for property shapes.
fn decompose_nested(
    shape: &Shape<'_>,
    owner_id: &str,
    role: &str,
    path: &str,
) -> (String, Vec<ConstraintEntry>) {
    // Own param set: every direct constraint's (component, canonical-params)
    // pair, in declaration order - this shape's content identity, distinct
    // from the IDs of shapes nested *inside* those constraints (computed
    // after, using this shape's own ID as their owner).
    let own_params: Vec<String> = shape
        .constraints
        .iter()
        .map(|c| {
            // Nested-shape constraints' own canonical params depend on their
            // children's IDs, which in turn depend on this shape's ID - so
            // for the *outer* shape's own identity, fold in only the
            // component IRI, not a recursively-unstable placeholder. This
            // matches "property_shape_id depends on path + own param set",
            // where "param set" is this shape's declared constraints as
            // written, not a value that depends on IDs computed later.
            component_iri(c).to_string()
        })
        .collect();
    let closed_marker = shape
        .closed
        .as_ref()
        .map(|c| {
            format!(
                "closed:{}",
                c.ignored_properties
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .unwrap_or_default();
    let shape_id = hash_parts(
        "shape",
        &[
            "content",
            owner_id,
            role,
            path,
            &own_params.join("\u{0}"),
            &shape.property_shapes.len().to_string(),
            &closed_marker,
        ],
    );

    let mut entries = Vec::new();
    for constraint in &shape.constraints {
        entries.push(build_constraint_entry(constraint, &shape_id, None));
    }
    if let Some(closed) = &shape.closed {
        entries.push(build_closed_entry(closed, &shape_id, None));
    }
    for (idx, prop_shape) in shape.property_shapes.iter().enumerate() {
        let prop_path = prop_shape
            .path
            .as_ref()
            .map(canonical_path)
            .unwrap_or_else(|| format!("<unpathed:{idx}>"));
        let (prop_shape_id, mut prop_entries) =
            decompose_nested(prop_shape, &shape_id, "property", &prop_path);
        for entry in &mut prop_entries {
            if entry.owner_property_shape.is_none() {
                entry.owner_property_shape = Some(prop_shape_id.clone());
                entry.path = Some(prop_path.clone());
            }
        }
        entries.extend(prop_entries);
    }
    (shape_id, entries)
}

/// Builds one flat [`ConstraintEntry`] for `constraint`, owned by
/// `owner_id`. Recurses into nested shapes first (so their IDs are ready for
/// `constraint_parameters`'s `child_ids`) but does NOT flatten those nested
/// shapes' own constraints into the caller's list - they live under
/// `children`, per R-1's rule that logical constraints get one entry with a
/// recursive `children` array.
fn build_constraint_entry(
    constraint: &Constraint<'_>,
    owner_id: &str,
    path: Option<String>,
) -> ConstraintEntry {
    let (child_ids, children): (Vec<String>, Option<Vec<ConstraintEntry>>) = match constraint {
        Constraint::And(AndConstraint(shapes))
        | Constraint::Or(OrConstraint(shapes))
        | Constraint::Xone(XoneConstraint(shapes)) => {
            let role = match constraint {
                Constraint::And(_) => "and",
                Constraint::Or(_) => "or",
                _ => "xone",
            };
            let mut ids = Vec::new();
            let mut child_entries = Vec::new();
            for member in shapes {
                let (member_id, member_constraints) = decompose_nested(member, owner_id, role, "");
                ids.push(member_id.clone());
                child_entries.push(ConstraintEntry {
                    id: member_id,
                    component: "http://www.w3.org/ns/shacl#Shape",
                    path: None,
                    parameters: Value::Null,
                    owner_property_shape: None,
                    severity: member.severity.as_str().to_string(),
                    messages: member.message.iter().cloned().collect(),
                    children: Some(member_constraints.into_iter().collect()),
                });
            }
            (ids, Some(child_entries))
        }
        Constraint::Not(NotConstraint(inner)) | Constraint::Node(NodeConstraint(inner)) => {
            let role = matches!(constraint, Constraint::Not(_))
                .then_some("not")
                .unwrap_or("node");
            let (id, entries) = decompose_nested(inner, owner_id, role, "");
            (
                vec![id.clone()],
                Some(vec![ConstraintEntry {
                    id,
                    component: "http://www.w3.org/ns/shacl#Shape",
                    path: None,
                    parameters: Value::Null,
                    owner_property_shape: None,
                    severity: inner.severity.as_str().to_string(),
                    messages: inner.message.iter().cloned().collect(),
                    children: Some(entries),
                }]),
            )
        }
        Constraint::QualifiedValueShape(c) => {
            let (id, entries) = decompose_nested(&c.shape, owner_id, "qualifiedValueShape", "");
            (
                vec![id.clone()],
                Some(vec![ConstraintEntry {
                    id,
                    component: "http://www.w3.org/ns/shacl#Shape",
                    path: None,
                    parameters: Value::Null,
                    owner_property_shape: None,
                    severity: c.shape.severity.as_str().to_string(),
                    messages: c.shape.message.iter().cloned().collect(),
                    children: Some(entries),
                }]),
            )
        }
        _ => (Vec::new(), None),
    };

    let (parameters, canonical_params) = constraint_parameters(constraint, &child_ids);
    let component = component_iri(constraint);
    let id = hash_parts("constraint", &[owner_id, component, &canonical_params]);

    ConstraintEntry {
        id,
        component,
        path,
        parameters,
        owner_property_shape: None, // filled in by the caller when this is a property-shape entry
        severity: String::new(),    // filled in by decompose_shape (node-shape severity applies)
        messages: Vec::new(),
        children,
    }
}

/// `sh:closed`/`sh:ignoredProperties` live on [`Shape::closed`], not in
/// `Shape::constraints` - built as its own [`ConstraintEntry`] here so
/// decomposition doesn't silently drop it (its own component,
/// `ClosedConstraintComponent`, has no `Constraint` enum variant).
fn build_closed_entry(
    closed: &ClosedConstraint<'_>,
    owner_id: &str,
    path: Option<String>,
) -> ConstraintEntry {
    let ignored: Vec<&str> = closed
        .ignored_properties
        .iter()
        .map(|p| p.as_str())
        .collect();
    let canonical_params = ignored.join("\u{0}");
    let component = sh::CLOSED_CONSTRAINT_COMPONENT.as_str();
    ConstraintEntry {
        id: hash_parts("constraint", &[owner_id, component, &canonical_params]),
        component,
        path,
        parameters: json!({"closed": true, "ignoredProperties": ignored}),
        owner_property_shape: None,
        severity: String::new(),
        messages: Vec::new(),
        children: None,
    }
}

fn target_to_json(target: &Target<'_>) -> Value {
    match target {
        Target::Node(term) => json!({"type": "targetNode", "value": canonical_term(*term)}),
        Target::Class(n) => json!({"type": "targetClass", "value": n.to_string()}),
        Target::SubjectsOf(n) => json!({"type": "targetSubjectsOf", "value": n.as_str()}),
        Target::ObjectsOf(n) => json!({"type": "targetObjectsOf", "value": n.as_str()}),
        Target::Sparql { query, .. } => json!({"type": "sparqlTarget", "value": query}),
        Target::Advanced(n) => json!({"type": "advancedTarget", "value": n.to_string()}),
    }
}

/// Decomposes one top-level (named or anonymous) shape into R-1's JSON
/// shape object. `source` is this shape's declaring source name (R-3),
/// `None` until named multi-source input exists.
fn decompose_top_level_shape(shape: &Shape<'_>, source: Option<&str>) -> Value {
    let shape_id = match shape.node {
        NamedOrBlankNodeRef::NamedNode(n) => hash_parts("shape", &["named", n.as_str()]),
        NamedOrBlankNodeRef::BlankNode(_) => {
            // Rare: an anonymous top-level shape. No IRI identity to hash;
            // fall back to the same content-derived scheme used for nested
            // shapes, with the synthetic "root" owner.
            let (id, _) = decompose_nested(shape, "root", "root", "");
            id
        }
    };

    let mut flat_entries = Vec::new();
    for constraint in &shape.constraints {
        let mut entry = build_constraint_entry(constraint, &shape_id, None);
        entry.severity = shape.severity.as_str().to_string();
        entry.messages = shape.message.iter().cloned().collect();
        flat_entries.push(entry);
    }
    if let Some(closed) = &shape.closed {
        let mut entry = build_closed_entry(closed, &shape_id, None);
        entry.severity = shape.severity.as_str().to_string();
        entry.messages = shape.message.iter().cloned().collect();
        flat_entries.push(entry);
    }
    for (idx, prop_shape) in shape.property_shapes.iter().enumerate() {
        let prop_path = prop_shape
            .path
            .as_ref()
            .map(canonical_path)
            .unwrap_or_else(|| format!("<unpathed:{idx}>"));
        let (prop_shape_id, mut prop_entries) =
            decompose_nested(prop_shape, &shape_id, "property", &prop_path);
        for entry in &mut prop_entries {
            if entry.owner_property_shape.is_none() {
                entry.owner_property_shape = Some(prop_shape_id.clone());
                entry.path = Some(prop_path.clone());
            }
            if entry.severity.is_empty() {
                entry.severity = prop_shape.severity.as_str().to_string();
            }
        }
        flat_entries.extend(prop_entries);
    }

    let kind = if shape.is_property_shape() {
        "property"
    } else {
        "node"
    };
    // A blank-node top-level shape has no IRI; report the stable id instead
    // of a per-run blank-node label, which would defeat R-2's purpose.
    let iri = match shape.node {
        NamedOrBlankNodeRef::NamedNode(n) => n.as_str().to_string(),
        NamedOrBlankNodeRef::BlankNode(_) => shape_id.clone(),
    };

    json!({
        "iri": iri,
        "id": shape_id,
        "kind": kind,
        "sources": source.map(|s| vec![s]).unwrap_or_default(),
        "targets": shape.targets.iter().map(target_to_json).collect::<Vec<_>>(),
        "severity": shape.severity.as_str(),
        "deactivated": shape.deactivated,
        "constraints": flat_entries.iter().map(|e| e.to_json(source)).collect::<Vec<_>>(),
    })
}

/// Full R-1 decomposition of a parsed shapes graph: every top-level shape
/// (as returned by [`crate::parser::parse_shapes`]) plus aggregate stats.
/// `source`/`graph_triple_count` feed the `sources`/`stats.triples` fields;
/// pass `None`/`0` when source attribution (R-3) isn't wired up yet.
pub fn decompose_shapes(
    shapes: &[Shape<'_>],
    source: Option<&str>,
    graph_triple_count: usize,
) -> Value {
    let shape_values: Vec<Value> = shapes
        .iter()
        .map(|s| decompose_top_level_shape(s, source))
        .collect();

    let constraint_count: usize = shape_values
        .iter()
        .map(|s| {
            s.get("constraints")
                .and_then(|c| c.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        })
        .sum();

    json!({
        "shapes": shape_values,
        "stats": {
            "shapes": shapes.len(),
            "constraints": constraint_count,
            "triples": graph_triple_count,
        },
    })
}
