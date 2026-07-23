use std::fmt::Write as _;

use super::{Diagnostic, DiagnosticSeverity, Snippet, SnippetOrigin, Verdict};

const ANSI_RED: &str = "\x1b[31m";
const ANSI_YELLOW: &str = "\x1b[33m";
const ANSI_CYAN: &str = "\x1b[36m";
const ANSI_RESET: &str = "\x1b[0m";

fn severity_word(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
    }
}

fn severity_color(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => ANSI_RED,
        DiagnosticSeverity::Warning => ANSI_YELLOW,
        DiagnosticSeverity::Info => ANSI_CYAN,
    }
}

fn verdict_word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Conforms => "conforms",
        Verdict::Violates => "violates",
        Verdict::NotTargeted => "not targeted",
        Verdict::Vacuous => "vacuously conforms",
    }
}

/// Plain plural rule: `1 error`, `2 errors`, `0 errors`.
fn pluralize(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Renders one snippet: the origin line, the fenced turtle lines with the
/// caret annotation beneath the line containing `highlight`, or a `= note:`
/// fallback line if `highlight` is not found anywhere in `turtle`.
fn render_snippet(out: &mut String, diag: &Diagnostic, snippet: &Snippet, color: bool) {
    match snippet.origin {
        SnippetOrigin::DataGraph => {
            let focus = diag.focus_node.as_deref().unwrap_or("");
            writeln!(out, "  data graph: triple of focus node {focus}").unwrap();
        }
        SnippetOrigin::ShapesGraph => {
            let shape = diag.source_shape.as_deref().unwrap_or("");
            writeln!(out, "  shapes graph: declared by {shape}").unwrap();
        }
    }

    writeln!(out, "   |").unwrap();

    let lines: Vec<&str> = snippet.turtle.split('\n').collect();
    let match_line = lines
        .iter()
        .position(|line| line.contains(snippet.highlight.as_str()));

    for (idx, line) in lines.iter().enumerate() {
        writeln!(out, "   |  {line}").unwrap();
        if Some(idx) == match_line {
            let byte_col = line.find(snippet.highlight.as_str()).unwrap();
            let col = line[..byte_col].chars().count();
            let pad = " ".repeat(col);
            let carets = "^".repeat(snippet.highlight.chars().count());
            if color {
                writeln!(
                    out,
                    "   |  {pad}{}{carets}{ANSI_RESET} {}",
                    severity_color(diag.severity),
                    snippet.annotation
                )
                .unwrap();
            } else {
                writeln!(out, "   |  {pad}{carets} {}", snippet.annotation).unwrap();
            }
        }
    }

    writeln!(out, "   |").unwrap();

    if match_line.is_none() {
        writeln!(out, "   = note: {}", snippet.annotation).unwrap();
    }
}

/// Renders a single diagnostic block (header through the last `= help:`
/// line), without the trailing blank-line separator.
fn render_diagnostic(diag: &Diagnostic, color: bool) -> String {
    let mut out = String::new();

    if color {
        writeln!(
            out,
            "{}{}[{}]{ANSI_RESET}: {}",
            severity_color(diag.severity),
            severity_word(diag.severity),
            diag.code,
            diag.title
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "{}[{}]: {}",
            severity_word(diag.severity),
            diag.code,
            diag.title
        )
        .unwrap();
    }

    for snippet in &diag.snippets {
        render_snippet(&mut out, diag, snippet, color);
    }

    if let Some(component) = &diag.constraint_component {
        writeln!(out, "   = component: <{component}>").unwrap();
    }
    if let Some(expected) = &diag.expected {
        writeln!(out, "   = expected: {expected}").unwrap();
    }
    if let Some(actual) = &diag.actual {
        writeln!(out, "   = actual:   {actual}").unwrap();
    }
    if let Some(verdict) = diag.verdict {
        writeln!(out, "   = note: verdict: {}", verdict_word(verdict)).unwrap();
    }
    for note in &diag.notes {
        writeln!(out, "   = note: {note}").unwrap();
    }
    if let Some(help) = &diag.help {
        writeln!(out, "   = help: {help}").unwrap();
    }

    out
}

pub fn render_text(diags: &[Diagnostic], color: bool) -> String {
    let mut error_count = 0usize;
    let mut warning_count = 0usize;

    let blocks: Vec<String> = diags
        .iter()
        .map(|diag| {
            match diag.severity {
                DiagnosticSeverity::Error => error_count += 1,
                DiagnosticSeverity::Warning => warning_count += 1,
                DiagnosticSeverity::Info => {}
            }
            render_diagnostic(diag, color)
        })
        .collect();

    // Each block already ends with a trailing newline, so joining with an
    // extra "\n" separator inserts exactly one blank line between blocks.
    let mut out = blocks.join("\n");
    out.push('\n');

    let worst = if error_count > 0 {
        "error"
    } else if warning_count > 0 {
        "warning"
    } else {
        "info"
    };
    writeln!(
        out,
        "{worst}: {}, {}",
        pluralize(error_count, "error"),
        pluralize(warning_count, "warning")
    )
    .unwrap();

    out
}
