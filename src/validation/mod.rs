pub mod constraints;
pub mod dataset;
pub mod report;

use oxigraph::model::{Graph, NamedNodeRef, NamedOrBlankNodeRef, TermRef};
use std::collections::{HashMap, HashSet};

#[cfg(not(target_family = "wasm"))]
use rayon::prelude::*;

use crate::{
    core::{constraints::Constraint, path::Path, shape::Shape, target::Target},
    utils,
    validation::{
        dataset::ValidationDataset,
        report::{ValidationReport, ValidationResult},
    },
    vocab::sh,
    ShaclError,
};

pub type TargetResolutionCache<'a> = HashMap<Target<'a>, HashSet<TermRef<'a>>>;

pub fn build_target_cache<'a>(
    data_graph: &'a Graph,
    shapes: &'a [Shape<'a>],
) -> TargetResolutionCache<'a> {
    let mut cache = TargetResolutionCache::new();

    for shape in shapes {
        for &target in &shape.targets {
            cache
                .entry(target)
                .or_insert_with(|| target.resolve_target_for_given_graph(data_graph));
        }
    }

    cache
}

/// Validation behavior for individual constraint types.
pub trait Validate<'a> {
    /// Validates the constraint for the given focus/value context.
    fn validate(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError>;
}

/// Context for validating one focus/value traversal.
#[derive(Debug, Clone)]
pub struct ValidationContext<'a> {
    /// Original focus node.
    pub focus_node: TermRef<'a>,
    /// Traversed path when validating a property shape.
    pub path: Option<&'a Path<'a>>,
    /// Value nodes under validation.
    pub value_nodes: Vec<TermRef<'a>>,
    /// Trace entries for nested shape evaluation.
    pub trace: Vec<String>,
}

/// Builder for `ValidationResult`.
#[derive(Debug, Clone)]
pub struct ViolationBuilder<'a> {
    pub focus_node: TermRef<'a>,
    pub value: Option<TermRef<'a>>,
    pub constraint_messages: Vec<String>,
    pub constraint_component: Option<NamedNodeRef<'a>>,
    pub constraint_detail: Option<String>,
    pub trace: Vec<String>,
    pub details: Vec<ValidationResult<'a>>,
}

impl<'a> ViolationBuilder<'a> {
    pub fn new(focus_node: TermRef<'a>) -> Self {
        Self {
            focus_node,
            value: None,
            constraint_messages: Vec::new(),
            constraint_component: None,
            constraint_detail: None,
            trace: Vec::new(),
            details: Vec::new(),
        }
    }

    pub fn value(mut self, value: TermRef<'a>) -> Self {
        self.value = Some(value);
        self
    }

    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.constraint_messages.push(msg.into());
        self
    }

    pub fn messages(mut self, messages: impl IntoIterator<Item = String>) -> Self {
        self.constraint_messages.extend(messages);
        self
    }

    pub fn component(mut self, component: NamedNodeRef<'a>) -> Self {
        self.constraint_component = Some(component);
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.constraint_detail = Some(detail.into());
        self
    }

    pub fn trace(mut self, trace: Vec<String>) -> Self {
        self.trace = trace;
        self
    }

    pub fn trace_entry(mut self, entry: impl Into<String>) -> Self {
        self.trace.push(entry.into());
        self
    }

    pub fn details(mut self, details: Vec<ValidationResult<'a>>) -> Self {
        self.details = details;
        self
    }
}

/// Validates a graph against all provided shapes.
pub fn validate<'a>(
    validation_dataset: &'a ValidationDataset,
    shapes: &'a [Shape<'a>],
) -> ValidationReport<'a> {
    let mut report = ValidationReport::new();
    let target_cache = build_target_cache(validation_dataset.data_graph(), shapes);
    #[cfg(not(target_family = "wasm"))]
    let shape_reports: Vec<ValidationReport<'a>> = shapes
        .par_iter()
        .map(|shape| shape.validate_with_target_cache(validation_dataset, &target_cache))
        .collect();

    #[cfg(target_family = "wasm")]
    let shape_reports: Vec<ValidationReport<'a>> = shapes
        .iter()
        .map(|shape| shape.validate_with_target_cache(validation_dataset, &target_cache))
        .collect();

    for shape_report in shape_reports {
        report.merge(shape_report);
    }

    report
}

impl<'a> Shape<'a> {
    /// Validates a data graph against this shape.
    pub fn validate(&'a self, validation_dataset: &'a ValidationDataset) -> ValidationReport<'a> {
        self.validate_with_target_cache(validation_dataset, &TargetResolutionCache::new())
    }

    pub fn validate_with_target_cache(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        target_cache: &TargetResolutionCache<'a>,
    ) -> ValidationReport<'a> {
        let mut report = ValidationReport {
            conforms: true,
            results: Vec::new(),
        };

        if self.deactivated {
            return report;
        }

        let mut focus_nodes = HashSet::new();
        for &target in &self.targets {
            if let Some(cached_nodes) = target_cache.get(&target) {
                focus_nodes.extend(cached_nodes.iter().copied());
            } else {
                focus_nodes
                    .extend(target.resolve_target_for_given_graph(validation_dataset.data_graph()));
            }
        }

        let focus_nodes_vec: Vec<_> = focus_nodes.into_iter().collect();

        #[cfg(not(target_family = "wasm"))]
        let focus_reports: Vec<ValidationReport<'a>> = focus_nodes_vec
            .par_iter()
            .map(|&focus_node| {
                let mut node_report = ValidationReport::new();
                self.validate_focus_node(validation_dataset, focus_node, &mut node_report);
                node_report
            })
            .collect();

        #[cfg(target_family = "wasm")]
        let focus_reports: Vec<ValidationReport<'a>> = focus_nodes_vec
            .iter()
            .map(|&focus_node| {
                let mut node_report = ValidationReport::new();
                self.validate_focus_node(
                    validation_dataset.data_graph(),
                    focus_node,
                    &mut node_report,
                );
                node_report
            })
            .collect();

        for node_report in focus_reports {
            report.merge(node_report);
        }

        report
    }

    /// Validates one node against this shape, without target resolution.
    fn validate_node(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        node: NamedOrBlankNodeRef<'a>,
    ) -> bool {
        self.validate_node_report(validation_dataset, node).conforms
    }

    fn validate_node_report(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        node: NamedOrBlankNodeRef<'a>,
    ) -> ValidationReport<'a> {
        let mut report = ValidationReport {
            conforms: true,
            results: Vec::new(),
        };

        if self.deactivated {
            return report;
        }

        self.validate_focus_node(validation_dataset, node.into(), &mut report);

        report
    }

    /// Validates a focus node against this shape.
    fn validate_focus_node(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        report: &mut ValidationReport<'a>,
    ) {
        let value_nodes = self.get_value_nodes(validation_dataset, focus_node);
        self.validate_constraints_on_values(validation_dataset, focus_node, &value_nodes, report);
        self.validate_nested_property_shapes(validation_dataset, focus_node, &value_nodes, report);
        self.validate_closed_constraint(validation_dataset, focus_node, report);
    }

    /// Resolves value nodes for the current shape.
    fn get_value_nodes(
        &'a self,
        data_graph: &'a Graph,
        focus_node: TermRef<'a>,
    ) -> Vec<TermRef<'a>> {
        if let Some(path) = &self.path {
            if let Some(focus_as_node) = utils::term_to_named_or_blank(focus_node) {
                path.resolve_path_for_given_node(data_graph, &focus_as_node)
            } else {
                Vec::new()
            }
        } else {
            vec![focus_node]
        }
    }

    /// Validates all constraints on the given value nodes
    fn validate_constraints_on_values(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        value_nodes: &[TermRef<'a>],
        report: &mut ValidationReport<'a>,
    ) {
        for constraint in &self.constraints {
            self.validate_constraint(
                validation_dataset,
                focus_node,
                value_nodes,
                constraint,
                report,
            );
        }
    }

    /// Validates nested property shapes on value nodes
    fn validate_nested_property_shapes(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        _focus_node: TermRef<'a>,
        value_nodes: &[TermRef<'a>],
        report: &mut ValidationReport<'a>,
    ) {
        if self.property_shapes.is_empty() {
            return;
        }

        // Precompute sibling qualified shapes for disjoint checks.
        let mut sibling_qualified_shapes: std::collections::HashMap<usize, Vec<&'a Shape<'a>>> =
            std::collections::HashMap::new();

        for (ps_idx, property_shape) in self.property_shapes.iter().enumerate() {
            if !property_shape.constraints.is_empty() {
                for constraint in &property_shape.constraints {
                    if let Constraint::QualifiedValueShape(qvs) = constraint {
                        if qvs.qualified_value_shapes_disjoint {
                            let mut siblings: Vec<&'a Shape<'a>> = Vec::new();
                            for (other_idx, other_ps) in self.property_shapes.iter().enumerate() {
                                if ps_idx == other_idx {
                                    continue;
                                }
                                for other_constraint in &other_ps.constraints {
                                    if let Constraint::QualifiedValueShape(other_qvs) =
                                        other_constraint
                                    {
                                        siblings.push(&other_qvs.shape);
                                    }
                                }
                            }
                            sibling_qualified_shapes.insert(ps_idx, siblings);
                            break;
                        }
                    }
                }
            }
        }

        for value_node in value_nodes {
            for (ps_idx, property_shape) in self.property_shapes.iter().enumerate() {
                if let Some(siblings) = sibling_qualified_shapes.get(&ps_idx) {
                    self.validate_property_shape_with_disjoint(
                        validation_dataset,
                        property_shape,
                        *value_node,
                        siblings,
                        report,
                    );
                } else {
                    property_shape.validate_focus_node(validation_dataset, *value_node, report);
                }
            }
        }
    }

    /// Validates a property shape with disjoint qualified value constraints.
    fn validate_property_shape_with_disjoint(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        property_shape: &'a Shape<'a>,
        focus_node: TermRef<'a>,
        sibling_qualified_shapes: &[&'a Shape<'a>],
        report: &mut ValidationReport<'a>,
    ) {
        let value_nodes = property_shape.get_value_nodes(validation_dataset, focus_node);
        let mut qualified_conforming_count = 0;

        for constraint in &property_shape.constraints {
            if let Constraint::QualifiedValueShape(qvs) = constraint {
                if qvs.qualified_value_shapes_disjoint {
                    for &value_node in &value_nodes {
                        if let Some(value_as_node) = utils::term_to_named_or_blank(value_node) {
                            if qvs.shape.validate_node(validation_dataset, value_as_node) {
                                let mut conforms_to_sibling = false;
                                for sibling_shape in sibling_qualified_shapes {
                                    if sibling_shape
                                        .validate_node(validation_dataset, value_as_node)
                                    {
                                        conforms_to_sibling = true;
                                        break;
                                    }
                                }
                                if !conforms_to_sibling {
                                    qualified_conforming_count += 1;
                                }
                            }
                        }
                    }

                    if let Some(min) = qvs.qualified_min_count {
                        if qualified_conforming_count < min {
                            let builder = ViolationBuilder::new(focus_node)
                                .message(format!(
                                    "Qualified value shape: {} values conform (min: {})",
                                    qualified_conforming_count, min
                                ))
                                .component(sh::QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT)
                                .detail(format!("sh:qualifiedMinCount {}", min));

                            report.conforms = false;
                            report
                                .results
                                .push(property_shape.build_validation_result(builder));
                        }
                    }

                    if let Some(max) = qvs.qualified_max_count {
                        if qualified_conforming_count > max {
                            let builder = ViolationBuilder::new(focus_node)
                                .message(format!(
                                    "Qualified value shape: {} values conform (max: {})",
                                    qualified_conforming_count, max
                                ))
                                .component(sh::QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT)
                                .detail(format!("sh:qualifiedMaxCount {}", max));

                            report.conforms = false;
                            report
                                .results
                                .push(property_shape.build_validation_result(builder));
                        }
                    }
                    continue;
                }
            }

            property_shape.validate_constraint(
                validation_dataset,
                focus_node,
                &value_nodes,
                constraint,
                report,
            );
        }

        property_shape.validate_nested_property_shapes(
            validation_dataset,
            focus_node,
            &value_nodes,
            report,
        );
        property_shape.validate_closed_constraint(validation_dataset, focus_node, report);
    }

    /// Validates `sh:closed`.
    fn validate_closed_constraint(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        report: &mut ValidationReport<'a>,
    ) {
        let closed_constraint = match &self.closed {
            Some(c) => c,
            None => return,
        };

        let focus_as_node = match utils::term_to_named_or_blank(focus_node) {
            Some(node) => node,
            None => return,
        };

        let mut allowed_properties: HashSet<NamedNodeRef<'a>> = HashSet::new();
        for ignored_prop in &closed_constraint.ignored_properties {
            allowed_properties.insert(*ignored_prop);
        }
        for property_shape in &self.property_shapes {
            if let Some(path) = &property_shape.path {
                for predicate in utils::extract_direct_predicates(path) {
                    allowed_properties.insert(predicate);
                }
            }
        }

        let data_graph = validation_dataset.data_graph();
        for triple in data_graph.triples_for_subject(focus_as_node) {
            if !allowed_properties.contains(&triple.predicate) {
                self.add_violation_with_component(
                    focus_node,
                    Some(triple.object),
                    Some(&format!(
                        "Property {} is not allowed (closed shape)",
                        triple.predicate
                    )),
                    Some(sh::CLOSED_CONSTRAINT_COMPONENT),
                    report,
                );
            }
        }
    }

    /// Validates one constraint for the given values.
    fn validate_constraint(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        value_nodes: &[TermRef<'a>],
        constraint: &'a Constraint<'a>,
        report: &mut ValidationReport<'a>,
    ) {
        let violations = match constraint {
            Constraint::Class(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Datatype(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::NodeKind(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MinCount(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MaxCount(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MinExclusive(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MinInclusive(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MaxExclusive(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MaxInclusive(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MinLength(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::MaxLength(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Pattern(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::LanguageIn(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::UniqueLang(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Equals(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Disjoint(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::LessThan(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::LessThanOrEquals(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::HasValue(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::In(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Node(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::QualifiedValueShape(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::And(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Or(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Xone(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Not(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
            Constraint::Sparql(c) => c.validate(
                validation_dataset,
                focus_node,
                self.path.as_ref(),
                value_nodes,
                self,
            ),
        };

        if let Ok(violations) = violations {
            for violation in violations {
                report.conforms = false;
                report.results.push(violation);
            }
        }
    }

    /// Compares two literal terms with a custom predicate.
    pub fn compare_values<F>(a: TermRef<'a>, b: TermRef<'a>, predicate: F) -> bool
    where
        F: Fn(i32) -> bool,
    {
        match (a, b) {
            (TermRef::Literal(lit_a), TermRef::Literal(lit_b)) => {
                // Try to parse as numbers
                let num_a = lit_a.value().parse::<f64>();
                let num_b = lit_b.value().parse::<f64>();

                match (num_a, num_b) {
                    (Ok(na), Ok(nb)) => {
                        // Both are numbers - compare numerically
                        if na < nb {
                            predicate(-1)
                        } else if na > nb {
                            predicate(1)
                        } else {
                            predicate(0)
                        }
                    }
                    (Err(_), Err(_)) => {
                        // Both are non-numeric - compare as strings
                        let cmp = lit_a.value().cmp(lit_b.value());
                        predicate(match cmp {
                            std::cmp::Ordering::Less => -1,
                            std::cmp::Ordering::Equal => 0,
                            std::cmp::Ordering::Greater => 1,
                        })
                    }
                    _ => {
                        // One is numeric, one is not - incompatible types
                        false
                    }
                }
            }
            _ => false,
        }
    }

    /// Builds a ValidationResult from a ViolationBuilder
    ///
    /// This is used by constraint validators to create properly formatted violation results
    /// with the shape's messages, severity, and other metadata.
    pub fn build_validation_result(
        &'a self,
        builder: ViolationBuilder<'a>,
    ) -> ValidationResult<'a> {
        let mut messages = Vec::new();

        // Include all constraint-specific messages, then shape-level messages.
        if !builder.constraint_messages.is_empty() {
            messages.extend(builder.constraint_messages);
        }
        messages.extend(self.message.iter().cloned());

        if !messages.is_empty() {
            let mut unique_messages = HashSet::new();
            messages.retain(|msg| unique_messages.insert(msg.clone()));
        }

        ValidationResult {
            focus_node: builder.focus_node,
            source_shape: self.node,
            source_shape_name: self.name.clone(),
            source_constraint_component: builder.constraint_component,
            constraint_detail: builder.constraint_detail,
            severity: self.severity,
            result_path: self.path.clone(),
            value: builder.value,
            messages,
            trace: builder.trace,
            details: builder.details,
        }
    }

    /// Adds a violation to the report using a builder
    fn add_violation_from_builder(
        &'a self,
        builder: ViolationBuilder<'a>,
        report: &mut ValidationReport<'a>,
    ) {
        report.conforms = false;

        let mut messages = Vec::new();

        // Include all constraint-specific messages, then shape-level messages.
        if !builder.constraint_messages.is_empty() {
            messages.extend(builder.constraint_messages);
        }
        messages.extend(self.message.iter().cloned());

        if !messages.is_empty() {
            let mut unique_messages = HashSet::new();
            messages.retain(|msg| unique_messages.insert(msg.clone()));
        }

        let result = ValidationResult {
            focus_node: builder.focus_node,
            source_shape: self.node,
            source_shape_name: self.name.clone(),
            source_constraint_component: builder.constraint_component,
            constraint_detail: builder.constraint_detail,
            severity: self.severity,
            result_path: self.path.clone(),
            value: builder.value,
            messages,
            trace: builder.trace,
            details: builder.details,
        };

        report.results.push(result);
    }

    /// Adds a violation to the report
    #[allow(dead_code)]
    fn add_violation(
        &'a self,
        focus_node: TermRef<'a>,
        value: Option<TermRef<'a>>,
        constraint_message: Option<&str>,
        report: &mut ValidationReport<'a>,
    ) {
        let mut builder = ViolationBuilder::new(focus_node);
        if let Some(v) = value {
            builder = builder.value(v);
        }
        if let Some(msg) = constraint_message {
            builder = builder.message(msg);
        }
        self.add_violation_from_builder(builder, report);
    }

    /// Adds a violation to the report with a specific constraint component
    fn add_violation_with_component(
        &'a self,
        focus_node: TermRef<'a>,
        value: Option<TermRef<'a>>,
        constraint_message: Option<&str>,
        constraint_component: Option<NamedNodeRef<'a>>,
        report: &mut ValidationReport<'a>,
    ) {
        let mut builder = ViolationBuilder::new(focus_node);
        if let Some(v) = value {
            builder = builder.value(v);
        }
        if let Some(msg) = constraint_message {
            builder = builder.message(msg);
        }
        if let Some(comp) = constraint_component {
            builder = builder.component(comp);
        }
        self.add_violation_from_builder(builder, report);
    }
}
