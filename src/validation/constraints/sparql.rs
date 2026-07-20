use oxigraph::{
    model::{NamedNode, NamedOrBlankNodeRef, Term, TermRef, Variable},
    sparql::{QueryResults, SparqlEvaluator},
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

/// True when the query references the variable (as `?name` or `$name`, with a
/// proper token boundary). Substituting a variable a query never mentions
/// makes oxigraph fail the whole execution.
fn query_mentions_variable(query: &str, name: &str) -> bool {
    let bytes = query.as_bytes();
    for (idx, _) in query.match_indices(name) {
        if idx == 0 {
            continue;
        }
        let sigil = bytes[idx - 1];
        if sigil != b'?' && sigil != b'$' {
            continue;
        }
        match bytes.get(idx + name.len()) {
            Some(c) if c.is_ascii_alphanumeric() || *c == b'_' => continue,
            _ => return true,
        }
    }
    false
}

/// Rewrites a query so that `$this` behaves as genuinely pre-bound, for the
/// constructs oxigraph's `substitute_variable` cannot evaluate (see
/// `SparqlConstraint::needs_text_prebinding`): `BOUND($this)` becomes `true`
/// (a pre-bound variable is bound by definition, in every scope), and other
/// expression references to `$this` are replaced by the focus-node constant.
/// Pattern positions keep the variable and are handled by
/// `substitute_variable`, which evaluates them correctly. Returns `None` when
/// the query doesn't parse (the caller then surfaces the parse error through
/// the normal path).
fn rewrite_this_prebinding(
    query: &str,
    prefixes: &[(String, String)],
    focus_node: TermRef<'_>,
) -> Option<String> {
    use spargebra::algebra::{AggregateExpression, Expression, GraphPattern, OrderExpression};

    let mut parser = spargebra::SparqlParser::new();
    for (prefix, namespace) in prefixes {
        if let Ok(with_prefix) = parser
            .clone()
            .with_prefix(prefix.clone(), namespace.clone())
        {
            parser = with_prefix;
        }
    }
    let parsed = parser.parse_query(query).ok()?;

    // Blank nodes have no expression form in SPARQL; for them only the
    // BOUND() rewrite applies (comparing a blank-node focus in a FILTER is
    // not expressible anyway).
    let constant: Option<Expression> = match focus_node {
        TermRef::NamedNode(n) => Some(Expression::NamedNode(n.into_owned())),
        TermRef::Literal(l) => Some(Expression::Literal(l.into_owned())),
        TermRef::BlankNode(_) => None,
    };
    let this = spargebra::term::Variable::new_unchecked("this");

    fn rewrite_expr(
        expr: Expression,
        this: &spargebra::term::Variable,
        constant: &Option<Expression>,
    ) -> Expression {
        let rec = |e: Box<Expression>| Box::new(rewrite_expr(*e, this, constant));
        match expr {
            Expression::Bound(v) if v == *this => {
                Expression::Literal(oxigraph::model::Literal::from(true))
            }
            Expression::Variable(v) if v == *this => match constant {
                Some(c) => c.clone(),
                None => Expression::Variable(v),
            },
            Expression::Variable(_)
            | Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Bound(_) => expr,
            Expression::Or(a, b) => Expression::Or(rec(a), rec(b)),
            Expression::And(a, b) => Expression::And(rec(a), rec(b)),
            Expression::Equal(a, b) => Expression::Equal(rec(a), rec(b)),
            Expression::SameTerm(a, b) => Expression::SameTerm(rec(a), rec(b)),
            Expression::Greater(a, b) => Expression::Greater(rec(a), rec(b)),
            Expression::GreaterOrEqual(a, b) => Expression::GreaterOrEqual(rec(a), rec(b)),
            Expression::Less(a, b) => Expression::Less(rec(a), rec(b)),
            Expression::LessOrEqual(a, b) => Expression::LessOrEqual(rec(a), rec(b)),
            Expression::In(a, list) => Expression::In(
                rec(a),
                list.into_iter()
                    .map(|e| rewrite_expr(e, this, constant))
                    .collect(),
            ),
            Expression::Add(a, b) => Expression::Add(rec(a), rec(b)),
            Expression::Subtract(a, b) => Expression::Subtract(rec(a), rec(b)),
            Expression::Multiply(a, b) => Expression::Multiply(rec(a), rec(b)),
            Expression::Divide(a, b) => Expression::Divide(rec(a), rec(b)),
            Expression::UnaryPlus(e) => Expression::UnaryPlus(rec(e)),
            Expression::UnaryMinus(e) => Expression::UnaryMinus(rec(e)),
            Expression::Not(e) => Expression::Not(rec(e)),
            Expression::Exists(p) => {
                Expression::Exists(Box::new(rewrite_pattern(*p, this, constant)))
            }
            Expression::If(a, b, c) => Expression::If(rec(a), rec(b), rec(c)),
            Expression::Coalesce(list) => Expression::Coalesce(
                list.into_iter()
                    .map(|e| rewrite_expr(e, this, constant))
                    .collect(),
            ),
            Expression::FunctionCall(f, args) => Expression::FunctionCall(
                f,
                args.into_iter()
                    .map(|e| rewrite_expr(e, this, constant))
                    .collect(),
            ),
        }
    }

    fn rewrite_pattern(
        pattern: GraphPattern,
        this: &spargebra::term::Variable,
        constant: &Option<Expression>,
    ) -> GraphPattern {
        let rec = |p: Box<GraphPattern>| Box::new(rewrite_pattern(*p, this, constant));
        match pattern {
            GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => {
                pattern
            }
            GraphPattern::Join { left, right } => GraphPattern::Join {
                left: rec(left),
                right: rec(right),
            },
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => GraphPattern::LeftJoin {
                left: rec(left),
                right: rec(right),
                expression: expression.map(|e| rewrite_expr(e, this, constant)),
            },
            GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
                left: rec(left),
                right: rec(right),
            },
            GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
                expr: rewrite_expr(expr, this, constant),
                inner: rec(inner),
            },
            GraphPattern::Union { left, right } => GraphPattern::Union {
                left: rec(left),
                right: rec(right),
            },
            GraphPattern::Graph { name, inner } => GraphPattern::Graph {
                name,
                inner: rec(inner),
            },
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => GraphPattern::Extend {
                inner: rec(inner),
                variable,
                expression: rewrite_expr(expression, this, constant),
            },
            GraphPattern::Minus { left, right } => GraphPattern::Minus {
                left: rec(left),
                right: rec(right),
            },
            GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
                inner: rec(inner),
                expression: expression
                    .into_iter()
                    .map(|oe| match oe {
                        OrderExpression::Asc(e) => {
                            OrderExpression::Asc(rewrite_expr(e, this, constant))
                        }
                        OrderExpression::Desc(e) => {
                            OrderExpression::Desc(rewrite_expr(e, this, constant))
                        }
                    })
                    .collect(),
            },
            GraphPattern::Project { inner, variables } => GraphPattern::Project {
                inner: rec(inner),
                variables,
            },
            GraphPattern::Distinct { inner } => GraphPattern::Distinct { inner: rec(inner) },
            GraphPattern::Reduced { inner } => GraphPattern::Reduced { inner: rec(inner) },
            GraphPattern::Slice {
                inner,
                start,
                length,
            } => GraphPattern::Slice {
                inner: rec(inner),
                start,
                length,
            },
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => GraphPattern::Group {
                inner: rec(inner),
                variables,
                aggregates: aggregates
                    .into_iter()
                    .map(|(v, agg)| {
                        (
                            v,
                            match agg {
                                AggregateExpression::CountSolutions { distinct } => {
                                    AggregateExpression::CountSolutions { distinct }
                                }
                                AggregateExpression::FunctionCall {
                                    name,
                                    expr,
                                    distinct,
                                } => AggregateExpression::FunctionCall {
                                    name,
                                    expr: rewrite_expr(expr, this, constant),
                                    distinct,
                                },
                            },
                        )
                    })
                    .collect(),
            },
            GraphPattern::Service {
                name,
                inner,
                silent,
            } => GraphPattern::Service {
                name,
                inner: rec(inner),
                silent,
            },
        }
    }

    let rewritten = match parsed {
        spargebra::Query::Select {
            dataset,
            pattern,
            base_iri,
        } => spargebra::Query::Select {
            dataset,
            pattern: rewrite_pattern(pattern, &this, &constant),
            base_iri,
        },
        spargebra::Query::Ask {
            dataset,
            pattern,
            base_iri,
        } => spargebra::Query::Ask {
            dataset,
            pattern: rewrite_pattern(pattern, &this, &constant),
            base_iri,
        },
        other => other,
    };

    Some(rewritten.to_string())
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

        // Constants that don't depend on the current value node: parsed and
        // substituted once per call when the fast path applies. Mentions are
        // checked against the text actually parsed (the rewrite may have
        // eliminated $this entirely), since substituting a variable the query
        // doesn't contain fails the whole execution.
        let apply_constant_substitutions = |mut prepared: oxigraph::sparql::PreparedSparqlQuery,
                                            text: &str| {
            if query_mentions_variable(text, "this") {
                prepared = prepared
                    .substitute_variable(Variable::new_unchecked("this"), focus_node.into_owned());
            }
            if query_mentions_variable(text, "shapesGraph") {
                prepared = prepared.substitute_variable(
                    Variable::new_unchecked("shapesGraph"),
                    NamedNode::new_unchecked(dataset::SHAPES_GRAPH_IRI),
                );
            }
            if query_mentions_variable(text, "currentShape") {
                prepared = prepared.substitute_variable(
                    Variable::new_unchecked("currentShape"),
                    Term::from(shape.node.into_owned()),
                );
            }
            if let Some(path) = path {
                if let Some(predicate) = utils::extract_direct_predicates(path).into_iter().next() {
                    if query_mentions_variable(text, "PATH") {
                        prepared = prepared.substitute_variable(
                            Variable::new_unchecked("PATH"),
                            predicate.into_owned(),
                        );
                    }
                }
            }
            for (name, value) in &self.parameter_bindings {
                if query_mentions_variable(text, name) {
                    if let Ok(var) = Variable::new(name.as_str()) {
                        prepared = prepared.substitute_variable(var, value.into_owned());
                    }
                }
            }
            prepared
        };

        if let Some(path) = path {
            if let Some(predicate) = utils::extract_direct_predicates(path).into_iter().next() {
                base_bindings.push(("PATH".to_string(), format!("{}", predicate)));
            }
        }
        for (name, value) in &self.parameter_bindings {
            base_bindings.push((name.to_string(), format!("{}", value)));
        }

        // Fast path: the query is parsed once per call and `$this` is
        // pre-bound via variable substitution. Skipped when the query needs
        // text-based pre-binding (see `needs_text_prebinding`'s doc comment).
        let prepared_base = if self.needs_text_prebinding {
            None
        } else {
            match evaluator.clone().parse_query(query_text) {
                Ok(prepared) => Some(apply_constant_substitutions(prepared, query_text)),
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
            }
        };

        for maybe_value in run_once_targets {
            let mut bindings = base_bindings.clone();

            if let Some(value) = maybe_value {
                bindings.push(("value".to_string(), format!("{}", value)));
            }

            let mut prepared = match &prepared_base {
                Some(prepared_base) => prepared_base.clone(),
                None => {
                    // Slow path: rewrite the AST per focus node so BOUND($this)
                    // and $this-derived expressions see the pre-binding, then
                    // apply the pattern-position substitutions on the rewritten
                    // query as usual.
                    let bound_query =
                        rewrite_this_prebinding(query_text, &self.prefixes, focus_node)
                            .unwrap_or_else(|| query_text.to_string());
                    match evaluator.clone().parse_query(&bound_query) {
                        Ok(prepared) => apply_constant_substitutions(prepared, &bound_query),
                        Err(error) => {
                            let mut builder = ViolationBuilder::new(focus_node)
                                .message(format!("SPARQL parse error: {}", error))
                                .component(constraint_component(self))
                                .detail(format!(
                                    "SPARQL query: {}",
                                    bound_query.replace('\n', " ")
                                ));
                            if let Some(value) = maybe_value {
                                builder = builder.value(value);
                            }
                            violations.push(shape.build_validation_result(builder));
                            continue;
                        }
                    }
                }
            };
            if let Some(value) = maybe_value {
                if query_mentions_variable(query_text, "value") {
                    prepared = prepared
                        .substitute_variable(Variable::new_unchecked("value"), value.into_owned());
                }
            }

            let results = prepared.on_store(store.as_ref()).execute();
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
        }

        Ok(violations)
    }
}
