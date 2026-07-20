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

/// Rewrites a query so the given pre-bound variables behave as genuinely
/// bound constants, covering the constructs oxigraph's `substitute_variable`
/// cannot: non-projected variables (including everything in an ASK, whose
/// projection is empty), `BOUND($var)` (becomes `true` — a pre-bound variable
/// is bound in every scope by definition), and expressions deriving values
/// from a pre-bound variable. Constants are inlined into expression, triple-
/// pattern, property-path, and predicate positions. Blank-node terms are
/// excluded (a blank node in query syntax is a fresh scoped variable, not a
/// reference) — those stay variables for `substitute_variable` to handle when
/// projected. Returns `None` when the query doesn't parse (the caller then
/// surfaces the parse error through the normal path).
fn rewrite_prebindings(
    query: &str,
    prefixes: &[(String, String)],
    binding_list: &[(&str, Term)],
) -> Option<String> {
    use spargebra::algebra::{AggregateExpression, Expression, GraphPattern, OrderExpression};
    use spargebra::term::{NamedNodePattern, TermPattern, TriplePattern};

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

    struct Bindings {
        map: Vec<(spargebra::term::Variable, Term)>,
    }
    impl Bindings {
        fn get(&self, v: &spargebra::term::Variable) -> Option<&Term> {
            self.map.iter().find(|(var, _)| var == v).map(|(_, t)| t)
        }
        fn expr(&self, v: &spargebra::term::Variable) -> Option<Expression> {
            match self.get(v)? {
                Term::NamedNode(n) => Some(Expression::NamedNode(n.clone())),
                Term::Literal(l) => Some(Expression::Literal(l.clone())),
                _ => None,
            }
        }
        fn term(&self, v: &spargebra::term::Variable) -> Option<TermPattern> {
            match self.get(v)? {
                Term::NamedNode(n) => Some(TermPattern::NamedNode(n.clone())),
                Term::Literal(l) => Some(TermPattern::Literal(l.clone())),
                _ => None,
            }
        }
        fn named(&self, v: &spargebra::term::Variable) -> Option<NamedNodePattern> {
            match self.get(v)? {
                Term::NamedNode(n) => Some(NamedNodePattern::NamedNode(n.clone())),
                _ => None,
            }
        }
    }
    let b = Bindings {
        map: binding_list
            .iter()
            .filter(|(_, t)| !matches!(t, Term::BlankNode(_)))
            .map(|(name, t)| (spargebra::term::Variable::new_unchecked(*name), t.clone()))
            .collect(),
    };
    fn rewrite_term(tp: TermPattern, b: &Bindings) -> TermPattern {
        match tp {
            TermPattern::Variable(v) => match b.term(&v) {
                Some(c) => c,
                None => TermPattern::Variable(v),
            },
            other => other,
        }
    }

    fn rewrite_named(np: NamedNodePattern, b: &Bindings) -> NamedNodePattern {
        match np {
            NamedNodePattern::Variable(v) => match b.named(&v) {
                Some(c) => c,
                None => NamedNodePattern::Variable(v),
            },
            other => other,
        }
    }

    fn rewrite_expr(expr: Expression, b: &Bindings) -> Expression {
        let rec = |e: Box<Expression>| Box::new(rewrite_expr(*e, b));
        match expr {
            Expression::Bound(v) if b.get(&v).is_some() => {
                Expression::Literal(oxigraph::model::Literal::from(true))
            }
            Expression::Variable(v) => match b.expr(&v) {
                Some(c) => c,
                None => Expression::Variable(v),
            },
            Expression::NamedNode(_) | Expression::Literal(_) | Expression::Bound(_) => expr,
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
                list.into_iter().map(|e| rewrite_expr(e, b)).collect(),
            ),
            Expression::Add(a, b) => Expression::Add(rec(a), rec(b)),
            Expression::Subtract(a, b) => Expression::Subtract(rec(a), rec(b)),
            Expression::Multiply(a, b) => Expression::Multiply(rec(a), rec(b)),
            Expression::Divide(a, b) => Expression::Divide(rec(a), rec(b)),
            Expression::UnaryPlus(e) => Expression::UnaryPlus(rec(e)),
            Expression::UnaryMinus(e) => Expression::UnaryMinus(rec(e)),
            Expression::Not(e) => Expression::Not(rec(e)),
            Expression::Exists(p) => Expression::Exists(Box::new(rewrite_pattern(*p, b))),
            Expression::If(a, b, c) => Expression::If(rec(a), rec(b), rec(c)),
            Expression::Coalesce(list) => {
                Expression::Coalesce(list.into_iter().map(|e| rewrite_expr(e, b)).collect())
            }
            Expression::FunctionCall(f, args) => {
                Expression::FunctionCall(f, args.into_iter().map(|e| rewrite_expr(e, b)).collect())
            }
        }
    }

    fn rewrite_pattern(pattern: GraphPattern, b: &Bindings) -> GraphPattern {
        let rec = |p: Box<GraphPattern>| Box::new(rewrite_pattern(*p, b));
        match pattern {
            GraphPattern::Bgp { patterns } => GraphPattern::Bgp {
                patterns: patterns
                    .into_iter()
                    .map(|t: TriplePattern| TriplePattern {
                        subject: rewrite_term(t.subject, b),
                        predicate: rewrite_named(t.predicate, b),
                        object: rewrite_term(t.object, b),
                    })
                    .collect(),
            },
            GraphPattern::Path {
                subject,
                path,
                object,
            } => GraphPattern::Path {
                subject: rewrite_term(subject, b),
                path,
                object: rewrite_term(object, b),
            },
            GraphPattern::Values { .. } => pattern,
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
                expression: expression.map(|e| rewrite_expr(e, b)),
            },
            GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
                left: rec(left),
                right: rec(right),
            },
            GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
                expr: rewrite_expr(expr, b),
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
                expression: rewrite_expr(expression, b),
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
                        OrderExpression::Asc(e) => OrderExpression::Asc(rewrite_expr(e, b)),
                        OrderExpression::Desc(e) => OrderExpression::Desc(rewrite_expr(e, b)),
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
                                    expr: rewrite_expr(expr, b),
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
            pattern: rewrite_pattern(pattern, &b),
            base_iri,
        },
        spargebra::Query::Ask {
            dataset,
            pattern,
            base_iri,
        } => spargebra::Query::Ask {
            dataset,
            pattern: rewrite_pattern(pattern, &b),
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

        let path_predicate =
            path.and_then(|p| utils::extract_direct_predicates(p).into_iter().next());
        if let Some(predicate) = path_predicate {
            base_bindings.push(("PATH".to_string(), format!("{}", predicate)));
        }
        for (name, value) in &self.parameter_bindings {
            base_bindings.push((name.to_string(), format!("{}", value)));
        }

        // oxigraph's `substitute_variable` only works for variables in the
        // query's projection (an ASK projects nothing). A mentioned pre-bound
        // variable outside the projection forces the AST-rewrite path.
        let is_projected = |name: &str| self.projected_vars.iter().any(|v| v == name);
        let mut prebound_names: Vec<&str> =
            vec!["this", "value", "PATH", "shapesGraph", "currentShape"];
        for (name, _) in &self.parameter_bindings {
            prebound_names.push(name.as_str());
        }
        let fast_path_ok = !self.needs_text_prebinding
            && prebound_names
                .iter()
                .all(|name| !query_mentions_variable(query_text, name) || is_projected(name));

        // Term forms of every pre-bound constant that the query mentions,
        // excluding the per-target `value` (added per iteration).
        let mut constant_terms: Vec<(&str, Term)> = Vec::new();
        if query_mentions_variable(query_text, "this") {
            constant_terms.push(("this", focus_node.into_owned()));
        }
        if query_mentions_variable(query_text, "shapesGraph") {
            constant_terms.push((
                "shapesGraph",
                NamedNode::new_unchecked(dataset::SHAPES_GRAPH_IRI).into(),
            ));
        }
        if query_mentions_variable(query_text, "currentShape") {
            constant_terms.push(("currentShape", Term::from(shape.node.into_owned())));
        }
        if let Some(predicate) = path_predicate {
            if query_mentions_variable(query_text, "PATH") {
                constant_terms.push(("PATH", predicate.into_owned().into()));
            }
        }
        for (name, value) in &self.parameter_bindings {
            if query_mentions_variable(query_text, name) {
                constant_terms.push((name.as_str(), value.into_owned()));
            }
        }

        // Substitutes any binding the rewrite could not inline (blank nodes),
        // when the variable survived into the text and is substitutable.
        let substitute_blank_leftovers = |mut prepared: oxigraph::sparql::PreparedSparqlQuery,
                                          terms: &[(&str, Term)],
                                          text: &str| {
            for (name, term) in terms {
                if matches!(term, Term::BlankNode(_))
                    && is_projected(name)
                    && query_mentions_variable(text, name)
                {
                    prepared =
                        prepared.substitute_variable(Variable::new_unchecked(*name), term.clone());
                }
            }
            prepared
        };

        // Fast path: parse once per call, pre-bind via variable substitution.
        let prepared_base = if !fast_path_ok {
            None
        } else {
            match evaluator.clone().parse_query(query_text) {
                Ok(mut prepared) => {
                    for (name, term) in &constant_terms {
                        prepared = prepared
                            .substitute_variable(Variable::new_unchecked(*name), term.clone());
                    }
                    Some(prepared)
                }
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
                    // Slow path: inline every pre-bound constant into the AST
                    // per target, so non-projected variables, BOUND(), and
                    // derived expressions all see the binding.
                    let mut term_bindings = constant_terms.clone();
                    if let Some(value) = maybe_value {
                        if query_mentions_variable(query_text, "value") {
                            term_bindings.push(("value", value.into_owned()));
                        }
                    }
                    let bound_query =
                        rewrite_prebindings(query_text, &self.prefixes, &term_bindings)
                            .unwrap_or_else(|| query_text.to_string());
                    match evaluator.clone().parse_query(&bound_query) {
                        Ok(prepared) => {
                            substitute_blank_leftovers(prepared, &term_bindings, &bound_query)
                        }
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
            if prepared_base.is_some() {
                if let Some(value) = maybe_value {
                    if query_mentions_variable(query_text, "value") {
                        prepared = prepared.substitute_variable(
                            Variable::new_unchecked("value"),
                            value.into_owned(),
                        );
                    }
                }
            }

            let results = prepared.on_store(store.as_ref()).execute();
            match (&self.executable, results) {
                (SparqlExecutable::Select(_), Ok(QueryResults::Solutions(solutions))) => {
                    let data = validation_dataset.data();
                    for solution_result in solutions {
                        let Ok(solution) = solution_result else {
                            continue;
                        };

                        let result_bindings: Vec<(String, String)> = solution
                            .iter()
                            .map(|(var, term)| (var.as_str().to_string(), term.to_string()))
                            .collect();

                        // Per SHACL-SPARQL, a ?path solution binding becomes the
                        // result's sh:resultPath and a ?value binding its
                        // sh:value; both are anchored back into the data
                        // graph's term space.
                        let solution_path = solution
                            .get("path")
                            .and_then(|t| data.canonical_term(t.as_ref()))
                            .and_then(|t| match t {
                                TermRef::NamedNode(nn) => Some(nn),
                                _ => None,
                            });
                        let solution_value = solution
                            .get("value")
                            .and_then(|t| data.canonical_term(t.as_ref()));

                        let mut builder = ViolationBuilder::new(focus_node)
                            .component(constraint_component(self))
                            .detail(format!("SPARQL SELECT: {}", query_text.replace('\n', " ")));

                        if let Some(p) = solution_path {
                            builder = builder.result_path(
                                Path::new().add_element(crate::core::path::PathElement::Iri(p)),
                            );
                        }
                        if let Some(v) = solution_value {
                            builder = builder.value(v);
                        } else if let Some(value) = maybe_value {
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
