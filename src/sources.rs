//! Named multi-source shapes-graph loading with collision detection (SHACL
//! Manager capability requirement R-3, 2026-07-31).
//!
//! Merging several shapes files for one validation run (a base vocabulary
//! plus project-specific extensions, say) previously happened as a silent
//! RDF union: if the same shape IRI received different triples from two
//! sources, the merged shape silently became the union of both, and the
//! per-source provenance needed to explain that was already gone by the
//! time anyone noticed. [`detect_collisions`] restores that provenance as
//! two diagnostics - `D0001` (error) when a shape IRI's triples actually
//! differ across sources, `D0002` (info) when they're triple-for-triple
//! identical (harmless, but usually a sign of an accidentally-duplicated
//! file) - while [`merge_sources`] still produces the union graph so
//! validation/decomposition can proceed either way.

use std::collections::BTreeMap;

use oxigraph::model::Graph;
use serde_json::json;

use crate::decompose::decompose_shapes;
use crate::diagnostics::{diagnostic_to_json, Diagnostic, DiagnosticSeverity};
use crate::err::ShaclError;
use crate::parser::parse_shapes;

/// One shapes-graph source with an attached name: flows into collision
/// diagnostics here and into `decompose_shapes`'s `sources`/`source` fields.
pub struct NamedSource {
    pub name: String,
    pub graph: Graph,
}

/// Unions every source's triples into one graph, for validation/parsing to
/// proceed regardless of collisions - see [`detect_collisions`] for the
/// diagnostics that flag what got merged.
pub fn merge_sources(sources: &[NamedSource]) -> Graph {
    let mut merged = Graph::new();
    for source in sources {
        for triple in source.graph.iter() {
            merged.insert(triple);
        }
    }
    merged
}

/// Decomposes every source independently (so each shape's `source`/
/// `sources` fields correctly attribute it, rather than everything looking
/// like it came from one big pre-merged union) and concatenates the
/// results, alongside [`detect_collisions`]'s D0001/D0002 diagnostics.
/// Shared by the CLI's `decompose` subcommand and the MCP `decompose_shapes`
/// tool, so both surfaces produce identical output for identical input
/// instead of maintaining two orchestration copies that could drift.
pub fn decompose_with_collisions(sources: &[NamedSource]) -> Result<serde_json::Value, ShaclError> {
    let collisions = detect_collisions(sources);

    let mut all_shapes = Vec::new();
    let mut total_triples = 0usize;
    for source in sources {
        let parsed_shapes = parse_shapes(&source.graph).map_err(|e| {
            ShaclError::Parse(format!(
                "SHACL shapes error in source '{}': {}",
                source.name, e
            ))
        })?;
        let decomposed = decompose_shapes(&parsed_shapes, Some(&source.name), source.graph.len());
        total_triples += source.graph.len();
        if let Some(shapes) = decomposed["shapes"].as_array() {
            all_shapes.extend(shapes.iter().cloned());
        }
    }
    let constraint_count: usize = all_shapes
        .iter()
        .map(|s| {
            s.get("constraints")
                .and_then(|c| c.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        })
        .sum();
    let collision_json: Vec<serde_json::Value> =
        collisions.iter().map(diagnostic_to_json).collect();

    Ok(json!({
        "shapes": all_shapes,
        "stats": {
            "shapes": all_shapes.len(),
            "constraints": constraint_count,
            "triples": total_triples,
        },
        "collisions": collision_json,
    }))
}

/// Content fingerprint for one decomposed top-level shape: everything that
/// matters for "is this the same declaration" but nothing that varies with
/// blank-node labels (those never appear here - `decompose_shapes` already
/// resolved every property shape / nested logical constraint down to
/// content-derived IDs, which is exactly what makes this comparison safe
/// across independently-parsed sources).
fn shape_fingerprint(shape_json: &serde_json::Value) -> String {
    fn collect_ids(entries: &serde_json::Value, out: &mut Vec<String>) {
        for entry in entries.as_array().into_iter().flatten() {
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                out.push(id.to_string());
            }
            if let Some(children) = entry.get("children") {
                collect_ids(children, out);
            }
        }
    }
    let mut constraint_ids = Vec::new();
    collect_ids(&shape_json["constraints"], &mut constraint_ids);
    constraint_ids.sort();

    let mut targets: Vec<String> = shape_json["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| t.to_string())
        .collect();
    targets.sort();

    format!(
        "kind={}\u{0}severity={}\u{0}deactivated={}\u{0}targets={}\u{0}constraints={}",
        shape_json["kind"],
        shape_json["severity"],
        shape_json["deactivated"],
        targets.join(","),
        constraint_ids.join(",")
    )
}

/// `D0001`/`D0002` for every named shape IRI declared (with a full
/// definition, i.e. present in `decompose_shapes`'s top-level `shapes`
/// array - see [`crate::decompose`]) by two or more sources. Comparison is
/// content-based via the same stable IDs `decompose_shapes` computes, not
/// raw triple text - so two sources with byte-identical shapes.ttl content
/// still correctly register as identical even though independently parsing
/// the same file twice assigns each parse fresh, different blank-node
/// labels for every property shape.
pub fn detect_collisions(sources: &[NamedSource]) -> Vec<Diagnostic> {
    // shape IRI -> one (source name, fingerprint) entry per source that
    // declares that IRI as a top-level shape.
    let mut by_iri: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

    for source in sources {
        let Ok(shapes) = parse_shapes(&source.graph) else {
            continue;
        };
        let decomposed = decompose_shapes(&shapes, Some(&source.name), source.graph.len());
        for shape_json in decomposed["shapes"].as_array().into_iter().flatten() {
            let Some(iri) = shape_json["iri"].as_str() else {
                continue;
            };
            by_iri
                .entry(iri.to_string())
                .or_default()
                .push((source.name.clone(), shape_fingerprint(shape_json)));
        }
    }

    let mut diagnostics = Vec::new();
    for (subject, defs) in by_iri {
        if defs.len() < 2 {
            continue;
        }
        let source_names: Vec<&str> = defs.iter().map(|(name, _)| name.as_str()).collect();
        let all_identical = defs.windows(2).all(|pair| pair[0].1 == pair[1].1);

        if all_identical {
            diagnostics.push(Diagnostic {
                code: "D0002",
                severity: DiagnosticSeverity::Info,
                title: "identical shape redefinition across sources".to_string(),
                constraint_component: None,
                snippets: Vec::new(),
                expected: None,
                actual: None,
                notes: vec![format!(
                    "declared identically in sources: {}",
                    source_names.join(", ")
                )],
                help: Some(
                    "harmless for validation (a union of identical triples is just those \
                     triples once), but usually means one source is a stale or duplicated \
                     copy of another."
                        .to_string(),
                ),
                focus_node: None,
                source_shape: Some(format!("<{subject}>")),
                path: None,
                verdict: None,
            });
        } else {
            let mut notes = vec![format!("declared in sources: {}", source_names.join(", "))];
            for (name, fingerprint) in &defs {
                notes.push(format!("{name}: {fingerprint}"));
            }
            diagnostics.push(Diagnostic {
                code: "D0001",
                severity: DiagnosticSeverity::Error,
                title: "shape IRI collision: conflicting definitions across sources".to_string(),
                constraint_component: None,
                snippets: Vec::new(),
                expected: None,
                actual: None,
                notes,
                help: Some(
                    "the merged shapes graph silently becomes the union of every source's \
                     triples for this IRI - give the shapes distinct IRIs, or reconcile them \
                     into one intentional definition in a single source."
                        .to_string(),
                ),
                focus_node: None,
                source_shape: Some(format!("<{subject}>")),
                path: None,
                verdict: None,
            });
        }
    }
    diagnostics
}
