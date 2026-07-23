//! Rustc-style diagnostics: a shared model rendered for terminals and as
//! NDJSON. Built as a post-processing layer over validation output; see
//! docs/superpowers/specs/2026-07-23-diagnostics-design.md.

mod derive;
mod explain_pass;
mod lint;
mod registry;
mod render_json;
mod render_text;

pub use derive::from_report;
pub use explain_pass::{explain_conformance, shape_target_nodes};
pub use lint::lint_shapes;
pub use registry::{all_entries, code_for_component, entry, RegistryEntry};
pub use render_json::{diagnostic_to_json, render_ndjson};
pub use render_text::render_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetOrigin {
    DataGraph,
    ShapesGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Conforms,
    Violates,
    NotTargeted,
    Vacuous,
}

/// A quoted, annotated piece of reconstructed Turtle. `highlight` is the
/// exact substring of `turtle` the renderer underlines; `annotation` is the
/// caret-line message.
#[derive(Debug, Clone)]
pub struct Snippet {
    pub origin: SnippetOrigin,
    pub turtle: String,
    pub highlight: String,
    pub annotation: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub title: String,
    pub constraint_component: Option<String>,
    pub snippets: Vec<Snippet>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub notes: Vec<String>,
    pub help: Option<String>,
    pub focus_node: Option<String>,
    pub source_shape: Option<String>,
    pub path: Option<String>,
    pub verdict: Option<Verdict>,
}

/// Deterministic output order: code, then focus node.
pub fn sort_diagnostics(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| {
        a.code
            .cmp(b.code)
            .then_with(|| a.focus_node.cmp(&b.focus_node))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(code: &'static str, focus: &str) -> Diagnostic {
        Diagnostic {
            code,
            severity: DiagnosticSeverity::Warning,
            title: String::new(),
            constraint_component: None,
            snippets: Vec::new(),
            expected: None,
            actual: None,
            notes: Vec::new(),
            help: None,
            focus_node: Some(focus.to_string()),
            source_shape: None,
            path: None,
            verdict: None,
        }
    }

    #[test]
    fn sorts_by_code_then_focus() {
        let mut v = vec![d("V0007", "b"), d("L0001", "z"), d("V0007", "a")];
        sort_diagnostics(&mut v);
        let keys: Vec<_> = v
            .iter()
            .map(|x| (x.code, x.focus_node.clone().unwrap()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("L0001", "z".to_string()),
                ("V0007", "a".to_string()),
                ("V0007", "b".to_string())
            ]
        );
    }

    #[test]
    fn severity_orders_error_first() {
        assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Warning);
        assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Info);
    }
}
