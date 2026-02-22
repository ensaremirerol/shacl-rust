use oxigraph::{
    model::{NamedOrBlankNodeRef, TermRef},
    sparql::{QueryResults, SparqlEvaluator},
};
use spargebra::{algebra::GraphPattern, Query, SparqlParser};

use crate::{
    core::{
        constraints::{SparqlConstraint, SparqlExecutable},
        path::Path,
        shape::Shape,
    },
    utils,
    validation::{
        dataset::{self, ValidationDataset},
        Validate, ValidationResult, ViolationBuilder,
    },
    vocab::sh,
    ShaclError,
};

fn constraint_component<'a>(c: &'a SparqlConstraint<'a>) -> oxigraph::model::NamedNodeRef<'a> {
    if let Some(NamedOrBlankNodeRef::NamedNode(component)) = c.source_constraint_component {
        component
    } else {
        sh::SPARQL_CONSTRAINT_COMPONENT
    }
}

fn unsupported_in_pattern(
    pattern: &GraphPattern,
    remaining_select_projects: usize,
) -> Option<&'static str> {
    match pattern {
        GraphPattern::Minus { .. } => Some("MINUS is not supported for SHACL pre-binding"),
        GraphPattern::Service { .. } => Some("SERVICE is not supported for SHACL pre-binding"),
        GraphPattern::Project { .. } if remaining_select_projects == 0 => {
            Some("Nested SELECT is not supported for SHACL pre-binding")
        }
        GraphPattern::Join { left, right } | GraphPattern::Union { left, right } => {
            unsupported_in_pattern(left, remaining_select_projects)
                .or_else(|| unsupported_in_pattern(right, remaining_select_projects))
        }
        GraphPattern::LeftJoin { left, right, .. } => {
            unsupported_in_pattern(left, remaining_select_projects)
                .or_else(|| unsupported_in_pattern(right, remaining_select_projects))
        }
        GraphPattern::Lateral { left, right } => {
            unsupported_in_pattern(left, remaining_select_projects)
                .or_else(|| unsupported_in_pattern(right, remaining_select_projects))
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => {
            unsupported_in_pattern(inner, remaining_select_projects)
        }
        GraphPattern::Project { inner, .. } => {
            unsupported_in_pattern(inner, remaining_select_projects.saturating_sub(1))
        }
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => None,
    }
}

fn unsupported_prebinding_construct(
    query: &str,
    prefixes: &[(String, String)],
) -> Option<&'static str> {
    let mut parser = SparqlParser::new();
    for (prefix, namespace) in prefixes {
        if let Ok(with_prefix) = parser
            .clone()
            .with_prefix(prefix.clone(), namespace.clone())
        {
            parser = with_prefix;
        }
    }

    let parsed = match parser.parse_query(query) {
        Ok(parsed) => parsed,
        Err(_) => return None,
    };

    let (pattern, remaining_select_projects) = match parsed {
        Query::Select { pattern, .. } => (pattern, 1),
        Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => (pattern, 0),
    };

    unsupported_in_pattern(&pattern, remaining_select_projects)
}

fn normalize_binding_value(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('<') && value.ends_with('>') {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn apply_message_bindings(
    template: &str,
    context_bindings: &[(String, String)],
    result_bindings: &[(String, String)],
) -> String {
    let mut rendered = template.to_string();

    for (var, value) in context_bindings.iter().chain(result_bindings.iter()) {
        let normalized = normalize_binding_value(value);
        rendered = rendered.replace(&format!("{{?{}}}", var), &normalized);
        rendered = rendered.replace(&format!("{{${}}}", var), &normalized);
    }

    rendered
}

fn render_messages_for_solution(
    messages: &[String],
    context_bindings: &[(String, String)],
    result_bindings: &[(String, String)],
) -> Vec<String> {
    messages
        .iter()
        .map(|m| apply_message_bindings(m, context_bindings, result_bindings))
        .collect()
}

impl<'a> Validate<'a> for SparqlConstraint<'a> {
    fn validate(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();

        let store = validation_dataset.store();

        let mut evaluator = SparqlEvaluator::new();
        for (prefix, namespace) in &self.prefixes {
            if let Ok(with_prefix) = evaluator
                .clone()
                .with_prefix(prefix.clone(), namespace.clone())
            {
                evaluator = with_prefix;
            }
        }

        let mut run_once_targets: Vec<Option<TermRef<'a>>> = Vec::new();
        if path.is_some() {
            if value_nodes.is_empty() {
                run_once_targets.push(None);
            } else {
                for &value in value_nodes {
                    run_once_targets.push(Some(value));
                }
            }
        } else if self.source_constraint_component.is_some() {
            run_once_targets.push(Some(focus_node));
        } else {
            run_once_targets.push(None);
        }

        let query_text = self.executable.query();

        if let Some(reason) = unsupported_prebinding_construct(query_text, &self.prefixes) {
            let mut builder = ViolationBuilder::new(focus_node)
                .component(constraint_component(self))
                .detail(format!("{}: {}", reason, query_text.replace('\n', " ")));

            if self.messages.is_empty() {
                builder = builder.message("SPARQL pre-binding violation");
            } else {
                builder = builder.messages(self.messages.clone());
            }

            if let Some(value) = value_nodes.first().copied() {
                builder = builder.value(value);
            }

            violations.push(shape.build_validation_result(builder));
            return Ok(violations);
        }

        for maybe_value in run_once_targets {
            let mut bindings: Vec<(String, String)> = Vec::new();
            bindings.push(("this".to_string(), format!("{}", focus_node)));
            bindings.push((
                "shapesGraph".to_string(),
                format!("<{}>", dataset::SHAPES_GRAPH_IRI),
            ));
            bindings.push(("currentShape".to_string(), format!("{}", shape.node)));

            if let Some(value) = maybe_value {
                bindings.push(("value".to_string(), format!("{}", value)));
            }

            if let Some(path) = path {
                if let Some(predicate) = utils::extract_direct_predicates(path).into_iter().next() {
                    bindings.push(("PATH".to_string(), format!("{}", predicate)));
                }
            }

            for (name, value) in &self.parameter_bindings {
                bindings.push((name.clone(), format!("{}", value)));
            }

            let bound_query = utils::inject_values_bindings(query_text, &bindings);

            let prepared = match evaluator.clone().parse_query(&bound_query) {
                Ok(prepared) => prepared,
                Err(error) => {
                    let mut builder = ViolationBuilder::new(focus_node)
                        .message(format!("SPARQL parse error: {}", error))
                        .component(constraint_component(self))
                        .detail(format!("SPARQL query: {}", bound_query.replace('\n', " ")));
                    if let Some(value) = maybe_value {
                        builder = builder.value(value);
                    }
                    violations.push(shape.build_validation_result(builder));
                    continue;
                }
            };

            let results = prepared.on_store(store.as_ref()).execute();
            let violations_before = violations.len();
            match (&self.executable, results) {
                (SparqlExecutable::Select(_), Ok(QueryResults::Solutions(solutions))) => {
                    for solution_result in solutions {
                        let Ok(solution) = solution_result else {
                            continue;
                        };

                        let result_bindings: Vec<(String, String)> = solution
                            .iter()
                            .map(|(var, term)| (var.as_str().to_string(), term.to_string()))
                            .collect();

                        let mut builder = ViolationBuilder::new(focus_node)
                            .component(constraint_component(self))
                            .detail(format!("SPARQL SELECT: {}", bound_query.replace('\n', " ")));

                        if let Some(value) = maybe_value {
                            builder = builder.value(value);
                        }

                        if self.messages.is_empty() {
                            builder = builder.message("SPARQL SELECT constraint violation");
                        } else {
                            builder = builder.messages(render_messages_for_solution(
                                &self.messages,
                                &bindings,
                                &result_bindings,
                            ));
                        }

                        violations.push(shape.build_validation_result(builder));
                    }
                }
                (SparqlExecutable::Ask(_), Ok(QueryResults::Boolean(result))) => {
                    if !result {
                        let mut builder = ViolationBuilder::new(focus_node)
                            .component(constraint_component(self))
                            .detail(format!("SPARQL ASK: {}", bound_query.replace('\n', " ")));

                        if let Some(value) = maybe_value {
                            builder = builder.value(value);
                        }

                        if self.messages.is_empty() {
                            builder = builder.message("SPARQL ASK constraint violation");
                        } else {
                            builder = builder.messages(self.messages.clone());
                        }

                        violations.push(shape.build_validation_result(builder));
                    }
                }
                (_, Ok(_)) => {}
                (_, Err(error)) => {
                    let mut builder = ViolationBuilder::new(focus_node)
                        .component(constraint_component(self))
                        .message(format!("SPARQL execution error: {}", error))
                        .detail(format!("SPARQL query: {}", bound_query.replace('\n', " ")));
                    if let Some(value) = maybe_value {
                        builder = builder.value(value);
                    }
                    violations.push(shape.build_validation_result(builder));
                }
            }

            let has_this_var = query_text.contains("$this") || query_text.contains("?this");
            if violations.len() == violations_before && has_this_var {
                let rewritten_query =
                    utils::rewrite_this_binding_query(query_text, &format!("{}", focus_node));
                let fallback_prepared = evaluator.clone().parse_query(&rewritten_query);
                if let Ok(fallback_prepared) = fallback_prepared {
                    let fallback_results = fallback_prepared.on_store(store.as_ref()).execute();
                    match (&self.executable, fallback_results) {
                        (SparqlExecutable::Select(_), Ok(QueryResults::Solutions(solutions))) => {
                            for solution_result in solutions {
                                let Ok(solution) = solution_result else {
                                    continue;
                                };

                                let result_bindings: Vec<(String, String)> = solution
                                    .iter()
                                    .map(|(var, term)| (var.as_str().to_string(), term.to_string()))
                                    .collect();

                                let mut builder = ViolationBuilder::new(focus_node)
                                    .component(constraint_component(self))
                                    .detail(format!(
                                        "SPARQL SELECT (fallback): {}",
                                        rewritten_query.replace('\n', " ")
                                    ));

                                if let Some(value) = maybe_value {
                                    builder = builder.value(value);
                                }

                                if self.messages.is_empty() {
                                    builder = builder.message("SPARQL SELECT constraint violation");
                                } else {
                                    builder = builder.messages(render_messages_for_solution(
                                        &self.messages,
                                        &bindings,
                                        &result_bindings,
                                    ));
                                }

                                violations.push(shape.build_validation_result(builder));
                            }
                        }
                        (SparqlExecutable::Ask(_), Ok(QueryResults::Boolean(result))) => {
                            if !result {
                                let mut builder = ViolationBuilder::new(focus_node)
                                    .component(constraint_component(self))
                                    .detail(format!(
                                        "SPARQL ASK (fallback): {}",
                                        rewritten_query.replace('\n', " ")
                                    ));

                                if let Some(value) = maybe_value {
                                    builder = builder.value(value);
                                }

                                if self.messages.is_empty() {
                                    builder = builder.message("SPARQL ASK constraint violation");
                                } else {
                                    builder = builder.messages(self.messages.clone());
                                }

                                violations.push(shape.build_validation_result(builder));
                            }
                        }
                        _ => {}
                    }
                }

                let unresolved_prebinding = violations.len() == violations_before
                    && (query_text.contains("bound($this")
                        || query_text.contains("bound(?this")
                        || query_text.contains("UNION"));

                if unresolved_prebinding {
                    let mut builder = ViolationBuilder::new(focus_node)
                        .component(constraint_component(self))
                        .detail(format!(
                            "SPARQL pre-binding fallback: {}",
                            query_text.replace('\n', " ")
                        ));

                    if self.messages.is_empty() {
                        builder = builder.message("SPARQL pre-binding violation");
                    } else {
                        builder = builder.messages(self.messages.clone());
                    }

                    if let Some(value) = maybe_value {
                        builder = builder.value(value);
                    }

                    violations.push(shape.build_validation_result(builder));
                }
            }
        }

        Ok(violations)
    }
}
