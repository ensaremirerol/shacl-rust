use oxigraph::model::{
    BlankNode, Graph, Literal, NamedNode, NamedNodeRef, NamedOrBlankNode, NamedOrBlankNodeRef,
    Term, TermRef, Triple,
};
use std::fmt::{Display, Formatter};

use crate::{vocab::sh, Path};

/// Validation report for a SHACL run.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationReport<'a> {
    /// Overall conformance.
    pub conforms: bool,
    /// Collected results.
    pub results: Vec<ValidationResult<'a>>,
}

/// One validation result.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult<'a> {
    /// Focus node.
    pub focus_node: TermRef<'a>,
    /// Source shape.
    pub source_shape: NamedOrBlankNodeRef<'a>,
    /// Optional source shape name.
    pub source_shape_name: Option<String>,
    /// Constraint component.
    pub source_constraint_component: Option<NamedNodeRef<'a>>,
    /// Human-readable constraint detail.
    pub constraint_detail: Option<String>,
    /// Result severity.
    pub severity: NamedNodeRef<'a>,
    /// Property path when available.
    pub result_path: Option<Path<'a>>,
    /// Value associated with the result.
    pub value: Option<TermRef<'a>>,
    /// Messages.
    pub messages: Vec<String>,
    /// Nested evaluation trace.
    pub trace: Vec<String>,
    /// Nested results.
    pub details: Vec<ValidationResult<'a>>,
}

impl<'a> ValidationReport<'a> {
    /// Creates an empty conforming report.
    pub fn new() -> Self {
        ValidationReport {
            conforms: true,
            results: Vec::new(),
        }
    }

    /// Returns the number of results.
    pub fn violation_count(&self) -> usize {
        self.results.len()
    }

    /// Returns results filtered by severity.
    pub fn violations_by_severity(&self, severity: NamedNodeRef<'a>) -> Vec<&ValidationResult<'a>> {
        self.results
            .iter()
            .filter(|r| r.severity == severity)
            .collect()
    }

    pub fn merge(&mut self, other: ValidationReport<'a>) {
        if !other.conforms {
            self.conforms = false;
        }
        self.results.extend(other.results);
    }

    /// Converts the report to an RDF graph.
    pub fn to_graph(&self) -> Graph {
        let mut graph = Graph::new();

        let report_node = BlankNode::default();
        let report_subject = NamedOrBlankNode::from(report_node);
        graph.insert(&Triple::new(
            report_subject.clone(),
            NamedNode::from(oxigraph::model::vocab::rdf::TYPE),
            Term::from(NamedNode::from(sh::VALIDATION_REPORT)),
        ));

        graph.insert(&Triple::new(
            report_subject.clone(),
            NamedNode::from(sh::CONFORMS),
            Term::from(Literal::from(self.conforms)),
        ));

        for result in &self.results {
            let result_subject = Self::add_validation_result_to_graph(&mut graph, result);
            graph.insert(&Triple::new(
                report_subject.clone(),
                NamedNode::from(sh::DETAIL),
                Term::from(result_subject),
            ));
        }

        graph
    }

    /// Adds one result to the graph and returns its subject node.
    fn add_validation_result_to_graph(
        graph: &mut Graph,
        result: &ValidationResult<'a>,
    ) -> NamedOrBlankNode {
        let result_node = BlankNode::default();
        let result_subject = NamedOrBlankNode::from(result_node);

        graph.insert(&Triple::new(
            result_subject.clone(),
            NamedNode::from(oxigraph::model::vocab::rdf::TYPE),
            Term::from(NamedNode::from(sh::VALIDATION_RESULT)),
        ));

        graph.insert(&Triple::new(
            result_subject.clone(),
            NamedNode::from(sh::FOCUS_NODE),
            Term::from(result.focus_node),
        ));

        graph.insert(&Triple::new(
            result_subject.clone(),
            NamedNode::from(sh::RESULT_SEVERITY),
            Term::from(NamedNode::from(result.severity)),
        ));

        graph.insert(&Triple::new(
            result_subject.clone(),
            NamedNode::from(sh::SOURCE_SHAPE),
            Term::from(result.source_shape),
        ));

        if let Some(component) = result.source_constraint_component {
            graph.insert(&Triple::new(
                result_subject.clone(),
                NamedNode::from(sh::SOURCE_CONSTRAINT_COMPONENT),
                Term::from(NamedNode::from(component)),
            ));
        }

        if let Some(value) = result.value {
            graph.insert(&Triple::new(
                result_subject.clone(),
                NamedNode::from(sh::VALUE),
                Term::from(value),
            ));
        }

        if let Some(ref path) = result.result_path {
            if let Some(crate::core::path::PathElement::Iri(iri)) = path.get_elements().first() {
                graph.insert(&Triple::new(
                    result_subject.clone(),
                    NamedNode::from(sh::RESULT_PATH),
                    Term::from(NamedNode::from(*iri)),
                ));
            }
        }

        for message in &result.messages {
            graph.insert(&Triple::new(
                result_subject.clone(),
                NamedNode::from(sh::RESULT_MESSAGE),
                Term::from(Literal::from(message.clone())),
            ));
        }

        if !result.trace.is_empty() {
            for trace_entry in &result.trace {
                graph.insert(&Triple::new(
                    result_subject.clone(),
                    NamedNode::from(sh::DETAIL),
                    Term::from(Literal::from(trace_entry.clone())),
                ));
            }
        }

        if !result.details.is_empty() {
            for detail in &result.details {
                let detail_subject = Self::add_validation_result_to_graph(graph, detail);
                graph.insert(&Triple::new(
                    result_subject.clone(),
                    NamedNode::from(sh::DETAIL),
                    Term::from(detail_subject),
                ));
            }
        }

        result_subject
    }

    pub fn as_json(&self) -> serde_json::Value {
        serde_json::json!({
            "conforms": self.conforms,
            "results": self.results.iter().map(|r| r.as_json()).collect::<Vec<_>>(),
        })
    }
}

impl ValidationResult<'_> {
    pub fn as_json(&self) -> serde_json::Value {
        let mut result_obj = serde_json::json!({
            "focusNode": self.focus_node.to_string(),
            "sourceShape": self.source_shape.to_string(),
            "severity": self.severity.to_string(),
        });

        if let Some(ref path) = self.result_path {
            result_obj["resultPath"] = serde_json::json!(path.to_string());
        }
        if let Some(value) = self.value {
            result_obj["value"] = serde_json::json!(value.to_string());
        }
        if !self.messages.is_empty() {
            result_obj["messages"] = serde_json::json!(self.messages);
        }
        if !self.trace.is_empty() {
            result_obj["trace"] = serde_json::json!(self.trace);
        }
        if !self.details.is_empty() {
            result_obj["details"] =
                serde_json::json!(self.details.iter().map(|d| d.as_json()).collect::<Vec<_>>());
        }
        result_obj
    }
}

impl<'a> Default for ValidationReport<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Display for ValidationReport<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n{}", "=".repeat(80))?;
        writeln!(f, "SHACL Validation Report")?;
        writeln!(f, "{}", "=".repeat(80))?;

        if self.conforms {
            write!(f, "\n✓ Data conforms to all shapes")?;
        } else {
            write!(f, "\n✗ Data does NOT conform to all shapes")?;
            write!(f, "\nViolations: {}", self.violation_count())?;

            let violations_count = self.violations_by_severity(sh::VIOLATION).len();
            let warnings_count = self.violations_by_severity(sh::WARNING).len();
            let info_count = self.violations_by_severity(sh::INFO).len();

            if violations_count > 0 {
                write!(f, "\n  - Violations: {}", violations_count)?;
            }
            if warnings_count > 0 {
                write!(f, "\n  - Warnings: {}", warnings_count)?;
            }
            if info_count > 0 {
                write!(f, "\n  - Info: {}", info_count)?;
            }

            writeln!(f, "\n\n{}", "-".repeat(80))?;
            writeln!(f, "Validation Results:")?;
            writeln!(f, "{}", "-".repeat(80))?;

            for (idx, result) in self.results.iter().enumerate() {
                writeln!(f, "\n[{}] Severity: {}", idx + 1, result.severity)?;
                writeln!(f, "  Focus Node: {}", result.focus_node)?;
                writeln!(f, "  Source Shape: {}", result.source_shape)?;

                if let Some(path) = &result.result_path {
                    writeln!(f, "  Result Path: {}", path)?;
                }

                if let Some(value) = result.value {
                    writeln!(f, "  Value: {}", value)?;
                }

                if !result.messages.is_empty() {
                    writeln!(f, "  Messages:")?;
                    for msg in &result.messages {
                        writeln!(f, "    - {}", msg)?;
                    }
                }

                if !result.details.is_empty() {
                    writeln!(f, "  Details:")?;
                    write_validation_result_details(f, &result.details, 4)?;
                }
            }
        }

        writeln!(f, "\n{}", "=".repeat(80))
    }
}

impl<'a> Display for ValidationResult<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Severity: {}", self.severity)?;
        writeln!(f, "Focus Node: {}", self.focus_node)?;
        writeln!(f, "Source Shape: {}", self.source_shape)?;

        if let Some(path) = &self.result_path {
            writeln!(f, "Result Path: {}", path)?;
        }

        if let Some(value) = self.value {
            writeln!(f, "Value: {}", value)?;
        }

        if !self.messages.is_empty() {
            writeln!(f, "Messages:")?;
            for msg in &self.messages {
                writeln!(f, "  - {}", msg)?;
            }
        }

        if !self.details.is_empty() {
            writeln!(f, "Details:")?;
            write_validation_result_details(f, &self.details, 2)?;
        }

        if !self.trace.is_empty() {
            writeln!(f, "Trace:")?;
            for trace_entry in &self.trace {
                writeln!(f, "  - {}", trace_entry)?;
            }
        }

        Ok(())
    }
}

fn write_validation_result_details(
    f: &mut Formatter<'_>,
    results: &[ValidationResult<'_>],
    indent: usize,
) -> std::fmt::Result {
    let pad = " ".repeat(indent);

    for (idx, result) in results.iter().enumerate() {
        writeln!(f, "{}- [{}] Severity: {}", pad, idx + 1, result.severity)?;
        writeln!(f, "{}  Focus Node: {}", pad, result.focus_node)?;
        writeln!(f, "{}  Source Shape: {}", pad, result.source_shape)?;

        if let Some(path) = &result.result_path {
            writeln!(f, "{}  Result Path: {}", pad, path)?;
        }

        if let Some(value) = result.value {
            writeln!(f, "{}  Value: {}", pad, value)?;
        }

        if !result.messages.is_empty() {
            writeln!(f, "{}  Messages:", pad)?;
            for msg in &result.messages {
                writeln!(f, "{}    - {}", pad, msg)?;
            }
        }

        if !result.details.is_empty() {
            writeln!(f, "{}  Details:", pad)?;
            write_validation_result_details(f, &result.details, indent + 4)?;
        }

        if !result.trace.is_empty() {
            writeln!(f, "{}  Trace:", pad)?;
            for trace_entry in &result.trace {
                writeln!(f, "{}    - {}", pad, trace_entry)?;
            }
        }
    }

    Ok(())
}
