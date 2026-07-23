use super::{Diagnostic, DiagnosticSeverity, SnippetOrigin, Verdict};
use serde_json::json;

pub fn diagnostic_to_json(d: &Diagnostic) -> serde_json::Value {
    let severity_str = match d.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
    };

    let verdict_str = d.verdict.map(|v| match v {
        Verdict::Conforms => "conforms",
        Verdict::Violates => "violates",
        Verdict::NotTargeted => "not-targeted",
        Verdict::Vacuous => "vacuous",
    });

    let snippets = d
        .snippets
        .iter()
        .map(|s| {
            let origin = match s.origin {
                SnippetOrigin::DataGraph => "data",
                SnippetOrigin::ShapesGraph => "shapes",
            };
            json!({
                "origin": origin,
                "turtle": s.turtle,
                "highlight": s.highlight,
                "annotation": s.annotation,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "code": d.code,
        "severity": severity_str,
        "title": d.title,
        "constraint_component": d.constraint_component,
        "snippets": snippets,
        "expected": d.expected,
        "actual": d.actual,
        "notes": d.notes,
        "help": d.help,
        "focus_node": d.focus_node,
        "source_shape": d.source_shape,
        "path": d.path,
        "verdict": verdict_str,
    })
}

pub fn render_ndjson(diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| {
            let json = diagnostic_to_json(d);
            format!("{json}\n")
        })
        .collect()
}
