use oxigraph::{
    model::{NamedNode, NamedOrBlankNodeRef, Term, TermRef, Variable},
    sparql::{PreparedSparqlQuery, QueryResults, SparqlEvaluator},
};

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

        let store = validation_dataset.store()?;

        let mut evaluator = SparqlEvaluator::new();
        for (prefix, namespace) in &self.prefixes {
            if let Ok(with_prefix) = evaluator
                .clone()
                .with_prefix(prefix.as_str(), namespace.as_str())
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

        // String forms of the bindings, kept only for message templates
        // ({$this}, {?value}, ...) rendered on violations.
        let mut base_bindings: Vec<(String, String)> = Vec::new();
        base_bindings.push(("this".to_string(), format!("{}", focus_node)));
        base_bindings.push((
            "shapesGraph".to_string(),
            format!("<{}>", dataset::SHAPES_GRAPH_IRI),
        ));
        base_bindings.push(("currentShape".to_string(), format!("{}", shape.node)));

        // The query is parsed once per call; per-target terms are pre-bound via
        // variable substitution, so the query text never changes.
        let prepared_base = match evaluator.clone().parse_query(query_text) {
            Ok(prepared) => prepared,
            Err(error) => {
                for maybe_value in run_once_targets {
                    let mut builder = ViolationBuilder::new(focus_node)
                        .message(format!("SPARQL parse error: {}", error))
                        .component(constraint_component(self))
                        .detail(format!("SPARQL query: {}", query_text.replace('\n', " ")));
                    if let Some(value) = maybe_value {
                        builder = builder.value(value);
                    }
                    violations.push(shape.build_validation_result(builder));
                }
                return Ok(violations);
            }
        };

        let mut prepared_base = prepared_base
            .substitute_variable(Variable::new_unchecked("this"), focus_node.into_owned())
            .substitute_variable(
                Variable::new_unchecked("shapesGraph"),
                NamedNode::new_unchecked(dataset::SHAPES_GRAPH_IRI),
            )
            .substitute_variable(
                Variable::new_unchecked("currentShape"),
                Term::from(shape.node.into_owned()),
            );

        if let Some(path) = path {
            if let Some(predicate) = utils::extract_direct_predicates(path).into_iter().next() {
                base_bindings.push(("PATH".to_string(), format!("{}", predicate)));
                prepared_base = prepared_base
                    .substitute_variable(Variable::new_unchecked("PATH"), predicate.into_owned());
            }
        }

        for (name, value) in &self.parameter_bindings {
            base_bindings.push((name.to_string(), format!("{}", value)));
            if let Ok(var) = Variable::new(name.as_str()) {
                prepared_base = prepared_base.substitute_variable(var, value.into_owned());
            }
        }

        // The $this fallback query is also constant per call; parsed lazily at
        // most once, only if a target actually needs it.
        let mut fallback: Option<Option<(PreparedSparqlQuery, String)>> = None;

        for maybe_value in run_once_targets {
            let mut bindings = base_bindings.clone();

            if let Some(value) = maybe_value {
                bindings.push(("value".to_string(), format!("{}", value)));
            }

            let mut prepared = prepared_base.clone();
            if let Some(value) = maybe_value {
                prepared = prepared
                    .substitute_variable(Variable::new_unchecked("value"), value.into_owned());
            }

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
                            .detail(format!("SPARQL SELECT: {}", query_text.replace('\n', " ")));

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
                            .detail(format!("SPARQL ASK: {}", query_text.replace('\n', " ")));

                        if let Some(value) = maybe_value {
                            builder = builder.value(value);
                        }

                        if self.messages.is_empty() {
                            builder = builder.message("SPARQL ASK constraint violation");
                        } else {
                            builder = builder.messages(self.messages.iter().cloned());
                        }

                        violations.push(shape.build_validation_result(builder));
                    }
                }
                (_, Ok(_)) => {}
                (_, Err(error)) => {
                    let mut builder = ViolationBuilder::new(focus_node)
                        .component(constraint_component(self))
                        .message(format!("SPARQL execution error: {}", error))
                        .detail(format!("SPARQL query: {}", query_text.replace('\n', " ")));
                    if let Some(value) = maybe_value {
                        builder = builder.value(value);
                    }
                    violations.push(shape.build_validation_result(builder));
                }
            }

            let has_this_var = query_text.contains("$this") || query_text.contains("?this");
            if violations.len() == violations_before && has_this_var {
                let fallback = fallback.get_or_insert_with(|| {
                    let rewritten_query =
                        utils::rewrite_this_binding_query(query_text, &format!("{}", focus_node));
                    evaluator
                        .clone()
                        .parse_query(&rewritten_query)
                        .ok()
                        .map(|prepared| (prepared, rewritten_query))
                });
                if let Some((fallback_prepared, rewritten_query)) = fallback {
                    let fallback_results =
                        fallback_prepared.clone().on_store(store.as_ref()).execute();
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
                        (SparqlExecutable::Ask(_), Ok(QueryResults::Boolean(result)))
                            if !result =>
                        {
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
                                builder = builder.messages(self.messages.iter().cloned());
                            }

                            violations.push(shape.build_validation_result(builder));
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
