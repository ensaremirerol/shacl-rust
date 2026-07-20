use std::collections::HashSet;

use oxigraph::model::{vocab::rdf, Graph, NamedOrBlankNodeRef, TermRef};
use spargebra::{
    algebra::{AggregateExpression, Expression, GraphPattern, OrderExpression},
    term::Variable,
    Query, SparqlParser,
};

use crate::{
    core::constraints::{Constraint, SparqlConstraint, SparqlExecutable},
    err::ShaclError,
    utils::{
        get_all_string_values, get_boolean_value, is_subclass_of, local_name_from_iri,
        parse_shacl_prefixes, term_to_named_or_blank,
    },
    vocab::sh,
};

/// A parsed executable plus whether it needs the slow, per-focus-node
/// text-based pre-binding path instead of oxigraph's `substitute_variable`.
struct ParsedExecutable {
    executable: SparqlExecutable,
    needs_text_prebinding: bool,
}

fn parse_executable<'a>(
    graph: &'a Graph,
    executable_node: NamedOrBlankNodeRef<'a>,
) -> Result<Option<ParsedExecutable>, ShaclError> {
    let sparql_executable = {
        if let Some(TermRef::Literal(lit)) =
            graph.object_for_subject_predicate(executable_node, sh::SELECT)
        {
            SparqlExecutable::Select(lit.value().to_string())
        } else if let Some(TermRef::Literal(lit)) =
            graph.object_for_subject_predicate(executable_node, sh::ASK)
        {
            SparqlExecutable::Ask(lit.value().to_string())
        } else {
            return Ok(None);
        }
    };

    let query_text = match &sparql_executable {
        SparqlExecutable::Select(query) | SparqlExecutable::Ask(query) => query,
    };
    let prefixes = parse_shacl_prefixes(graph, executable_node);

    if let Some(error) = unsupported_prebinding_construct(query_text, &prefixes) {
        return Err(ShaclError::Parse(format!(
            "Unsupported SPARQL construct for SHACL pre-binding: {}",
            error
        )));
    }

    let needs_text_prebinding = query_needs_text_prebinding(query_text, &prefixes);

    Ok(Some(ParsedExecutable {
        executable: sparql_executable,
        needs_text_prebinding,
    }))
}

/// True when the query uses `$this`/`?this` in a way that oxigraph's
/// `substitute_variable` cannot handle correctly: `BOUND($this)` does not see
/// a pre-substituted value as bound, and `BIND(... AS ?other)` expressions
/// referencing `$this` don't propagate its substituted value into `?other`.
/// Plain use of `$this` in triple/path patterns and direct comparisons (e.g.
/// `?this ex:p ?o`, `FILTER($this = ex:x)`) work fine and are not flagged, so
/// the common case keeps the fast, parse-once path.
fn query_needs_text_prebinding(query: &str, prefixes: &[(String, String)]) -> bool {
    let mut parser = SparqlParser::new();
    for (prefix, namespace) in prefixes {
        if let Ok(with_prefix) = parser
            .clone()
            .with_prefix(prefix.clone(), namespace.clone())
        {
            parser = with_prefix;
        }
    }
    let Ok(parsed) = parser.parse_query(query) else {
        return false;
    };
    let pattern = match &parsed {
        Query::Select { pattern, .. } | Query::Ask { pattern, .. } => pattern,
        Query::Construct { .. } | Query::Describe { .. } => return false,
    };
    let this = Variable::new_unchecked("this");
    pattern_has_bound_of(pattern, &this) || pattern_extend_derives_from(pattern, &this)
}

/// True if `BOUND(target)` appears anywhere in `pattern` (inside FILTER,
/// OPTIONAL's join condition, BIND, ORDER BY, or aggregate expressions,
/// including within nested `FILTER EXISTS { ... }` sub-patterns).
fn pattern_has_bound_of(pattern: &GraphPattern, target: &Variable) -> bool {
    match pattern {
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            pattern_has_bound_of(left, target) || pattern_has_bound_of(right, target)
        }
        GraphPattern::Lateral { left, right } => {
            pattern_has_bound_of(left, target) || pattern_has_bound_of(right, target)
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            pattern_has_bound_of(left, target)
                || pattern_has_bound_of(right, target)
                || expression
                    .as_ref()
                    .is_some_and(|e| expr_has_bound_of(e, target))
        }
        GraphPattern::Filter { expr, inner } => {
            expr_has_bound_of(expr, target) || pattern_has_bound_of(inner, target)
        }
        GraphPattern::Graph { inner, .. } => pattern_has_bound_of(inner, target),
        GraphPattern::Extend {
            inner, expression, ..
        } => expr_has_bound_of(expression, target) || pattern_has_bound_of(inner, target),
        GraphPattern::OrderBy { inner, expression } => {
            pattern_has_bound_of(inner, target)
                || expression.iter().any(|oe| {
                    let e = match oe {
                        OrderExpression::Asc(e) | OrderExpression::Desc(e) => e,
                    };
                    expr_has_bound_of(e, target)
                })
        }
        GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => pattern_has_bound_of(inner, target),
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            pattern_has_bound_of(inner, target)
                || aggregates.iter().any(|(_, agg)| match agg {
                    AggregateExpression::CountSolutions { .. } => false,
                    AggregateExpression::FunctionCall { expr, .. } => {
                        expr_has_bound_of(expr, target)
                    }
                })
        }
        GraphPattern::Service { inner, .. } => pattern_has_bound_of(inner, target),
    }
}

fn expr_has_bound_of(expr: &Expression, target: &Variable) -> bool {
    match expr {
        Expression::Bound(v) => v == target,
        Expression::Exists(pattern) => pattern_has_bound_of(pattern, target),
        Expression::Variable(_) | Expression::NamedNode(_) | Expression::Literal(_) => false,
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => expr_has_bound_of(a, target) || expr_has_bound_of(b, target),
        Expression::In(a, list) => {
            expr_has_bound_of(a, target) || list.iter().any(|e| expr_has_bound_of(e, target))
        }
        Expression::UnaryPlus(e) | Expression::UnaryMinus(e) | Expression::Not(e) => {
            expr_has_bound_of(e, target)
        }
        Expression::If(a, b, c) => {
            expr_has_bound_of(a, target)
                || expr_has_bound_of(b, target)
                || expr_has_bound_of(c, target)
        }
        Expression::Coalesce(list) => list.iter().any(|e| expr_has_bound_of(e, target)),
        Expression::FunctionCall(_, args) => args.iter().any(|e| expr_has_bound_of(e, target)),
    }
}

/// True if any `BIND(expr AS ?other)` in `pattern` has `expr` referencing
/// `target` (directly or via `BOUND`), which oxigraph's `substitute_variable`
/// cannot propagate into `?other`. Does not descend into `FILTER EXISTS { }`
/// sub-patterns for this check (an even rarer nesting this doesn't need to
/// cover).
fn pattern_extend_derives_from(pattern: &GraphPattern, target: &Variable) -> bool {
    match pattern {
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            pattern_extend_derives_from(left, target) || pattern_extend_derives_from(right, target)
        }
        GraphPattern::Lateral { left, right } => {
            pattern_extend_derives_from(left, target) || pattern_extend_derives_from(right, target)
        }
        GraphPattern::LeftJoin { left, right, .. } => {
            pattern_extend_derives_from(left, target) || pattern_extend_derives_from(right, target)
        }
        GraphPattern::Filter { inner, .. } | GraphPattern::Graph { inner, .. } => {
            pattern_extend_derives_from(inner, target)
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            (variable != target && expr_references(expression, target))
                || pattern_extend_derives_from(inner, target)
        }
        GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. }
        | GraphPattern::Service { inner, .. } => pattern_extend_derives_from(inner, target),
    }
}

fn expr_references(expr: &Expression, target: &Variable) -> bool {
    match expr {
        Expression::Variable(v) => v == target,
        Expression::Bound(v) => v == target,
        Expression::Exists(_) => false,
        Expression::NamedNode(_) | Expression::Literal(_) => false,
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => expr_references(a, target) || expr_references(b, target),
        Expression::In(a, list) => {
            expr_references(a, target) || list.iter().any(|e| expr_references(e, target))
        }
        Expression::UnaryPlus(e) | Expression::UnaryMinus(e) | Expression::Not(e) => {
            expr_references(e, target)
        }
        Expression::If(a, b, c) => {
            expr_references(a, target) || expr_references(b, target) || expr_references(c, target)
        }
        Expression::Coalesce(list) => list.iter().any(|e| expr_references(e, target)),
        Expression::FunctionCall(_, args) => args.iter().any(|e| expr_references(e, target)),
    }
}

/// Walks a query's algebra looking for constructs SHACL pre-binding can't support.
///
/// `at_top` tracks whether we're still within the chain of modifiers wrapping the
/// *outermost* query itself (its own Project/Filter/OrderBy/etc., however many layers
/// that happens to be) versus having crossed into a branch of a real combinator
/// (Join/Union/LeftJoin/Lateral) where a genuinely nested sub-SELECT could live. This
/// avoids hardcoding "how many Project layers does a top-level ASK vs. SELECT have" --
/// that's an internal representation detail of the SPARQL algebra library and not
/// stable across its versions (e.g. spargebra 0.4.6 started wrapping ASK's pattern in
/// an explicit `Project { variables: [], .. }`, where 0.4.5 didn't wrap it at all;
/// counting a fixed "remaining projects" budget per query type broke against that).
fn unsupported_in_pattern(
    pattern: &GraphPattern,
    at_top: bool,
    required_prebound_vars: &HashSet<Variable>,
) -> Option<&'static str> {
    match pattern {
        GraphPattern::Minus { .. } => Some("MINUS is not supported for SHACL pre-binding"),
        GraphPattern::Service { .. } => Some("SERVICE is not supported for SHACL pre-binding"),
        GraphPattern::Project { variables, inner } => {
            // Nested SELECT is only allowed if it explicitly projects all
            // pre-bound variables. Only the single outermost Project belongs
            // to the query itself; passing through it consumes `at_top`, so a
            // directly nested `{ SELECT ... }` is checked even without an
            // intervening Join.
            if !at_top {
                let projected_vars: HashSet<_> = variables.iter().cloned().collect();
                if !required_prebound_vars.is_subset(&projected_vars) {
                    return Some(
                        "Nested SELECT must explicitly project all pre-bound variables (e.g., $this)",
                    );
                }
            }
            unsupported_in_pattern(inner, false, required_prebound_vars)
        }
        // BIND must not assign into one of the pre-bound SHACL variables
        // (e.g. `BIND(true AS $this)`): the query would be rewriting the
        // very variable SHACL pre-binds, which is nonsensical and explicitly
        // disallowed.
        GraphPattern::Extend {
            inner, variable, ..
        } if required_prebound_vars.contains(variable) => {
            Some("BIND must not assign to a pre-bound SHACL variable (e.g. $this)")
        }
        // Still within the outer query's own modifier chain -- transparent regardless
        // of how many layers deep, since none of these introduce a new sub-query scope.
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => {
            unsupported_in_pattern(inner, at_top, required_prebound_vars)
        }
        // A real combination point: either branch from here on is a genuinely separate
        // (possibly nested-SELECT) sub-pattern, not part of the outer query's own chain.
        GraphPattern::Join { left, right } | GraphPattern::Union { left, right } => {
            unsupported_in_pattern(left, false, required_prebound_vars)
                .or_else(|| unsupported_in_pattern(right, false, required_prebound_vars))
        }
        GraphPattern::LeftJoin { left, right, .. } => {
            unsupported_in_pattern(left, false, required_prebound_vars)
                .or_else(|| unsupported_in_pattern(right, false, required_prebound_vars))
        }
        GraphPattern::Lateral { left, right } => {
            unsupported_in_pattern(left, false, required_prebound_vars)
                .or_else(|| unsupported_in_pattern(right, false, required_prebound_vars))
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

    let (pattern, prebound_vars) = match parsed {
        Query::Select { pattern, .. } | Query::Ask { pattern, .. } => {
            // $this is always pre-bound, for both SELECT and ASK queries.
            let mut vars = HashSet::new();
            vars.insert(Variable::new_unchecked("this"));
            (pattern, vars)
        }
        Query::Construct { pattern, .. } | Query::Describe { pattern, .. } => {
            (pattern, HashSet::new())
        }
    };

    unsupported_in_pattern(&pattern, true, &prebound_vars)
}

fn parse_direct_shape_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = Vec::new();
    let mut seen_sources = std::collections::HashSet::new();

    for sparql_term in graph.objects_for_subject_predicate(shape_node, sh::SPARQL) {
        let Some(executable_node) = term_to_named_or_blank(sparql_term) else {
            continue;
        };

        if !seen_sources.insert(executable_node) {
            continue;
        };

        let ParsedExecutable {
            executable,
            needs_text_prebinding,
        } = match parse_executable(graph, executable_node)? {
            Some(parsed) => parsed,
            None => continue,
        };

        constraints.push(Constraint::Sparql(SparqlConstraint {
            source_constraint: Some(executable_node),
            source_constraint_component: None,
            executable,
            messages: get_all_string_values(graph, executable_node, sh::MESSAGE),
            prefixes: parse_shacl_prefixes(graph, executable_node),
            parameter_bindings: Vec::new(),
            needs_text_prebinding,
        }));
    }

    if seen_sources.insert(shape_node) {
        let ParsedExecutable {
            executable,
            needs_text_prebinding,
        } = match parse_executable(graph, shape_node)? {
            Some(parsed) => parsed,
            None => return Ok(constraints),
        };
        constraints.push(Constraint::Sparql(SparqlConstraint {
            source_constraint: Some(shape_node),
            source_constraint_component: None,
            executable,
            messages: get_all_string_values(graph, shape_node, sh::MESSAGE),
            prefixes: parse_shacl_prefixes(graph, shape_node),
            parameter_bindings: Vec::new(),
            needs_text_prebinding,
        }));
    }

    Ok(constraints)
}

fn is_constraint_component_instance<'a>(
    graph: &'a Graph,
    component: NamedOrBlankNodeRef<'a>,
) -> bool {
    graph
        .objects_for_subject_predicate(component, rdf::TYPE)
        .filter_map(term_to_named_or_blank)
        .any(|component_type| {
            component_type == sh::CONSTRAINT_COMPONENT.into()
                || is_subclass_of(component_type, sh::CONSTRAINT_COMPONENT.into(), graph)
        })
}

fn parse_component_parameter_bindings<'a>(
    graph: &'a Graph,
    component: NamedOrBlankNodeRef<'a>,
    shape_node: NamedOrBlankNodeRef<'a>,
) -> Option<Vec<(String, TermRef<'a>)>> {
    let mut bindings = Vec::new();

    for parameter_term in graph.objects_for_subject_predicate(component, sh::PARAMETER) {
        let parameter_node = term_to_named_or_blank(parameter_term)?;

        let path = graph
            .object_for_subject_predicate(parameter_node, sh::PATH)
            .and_then(|t| match t {
                TermRef::NamedNode(nn) => Some(nn),
                _ => None,
            })?;

        let var_name = local_name_from_iri(path.as_str())?;
        let optional = get_boolean_value(graph, parameter_node, sh::OPTIONAL).unwrap_or(false);

        let mut values = graph.objects_for_subject_predicate(shape_node, path);
        if let Some(value) = values.next() {
            bindings.push((var_name, value));
        } else if !optional {
            return None;
        }
    }

    Some(bindings)
}

fn parse_component_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
    is_property_shape: bool,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = Vec::new();

    let mut validator_predicates = vec![sh::VALIDATOR];
    if is_property_shape {
        validator_predicates.push(sh::PROPERTY_VALIDATOR);
    } else {
        validator_predicates.push(sh::NODE_VALIDATOR);
    }

    let components: std::collections::HashSet<_> = graph
        .triples_for_predicate(sh::PARAMETER)
        .map(|triple| triple.subject)
        .collect();

    for component in components {
        if !is_constraint_component_instance(graph, component) {
            continue;
        }

        let Some(parameter_bindings) =
            parse_component_parameter_bindings(graph, component, shape_node)
        else {
            continue;
        };

        for predicate in &validator_predicates {
            for validator_term in graph.objects_for_subject_predicate(component, *predicate) {
                let Some(validator_node) = term_to_named_or_blank(validator_term) else {
                    continue;
                };

                let ParsedExecutable {
                    executable,
                    needs_text_prebinding,
                } = match parse_executable(graph, validator_node)? {
                    Some(parsed) => parsed,
                    None => continue,
                };
                constraints.push(Constraint::Sparql(SparqlConstraint {
                    source_constraint: Some(validator_node),
                    source_constraint_component: Some(component),
                    executable,
                    messages: get_all_string_values(graph, validator_node, sh::MESSAGE),
                    prefixes: parse_shacl_prefixes(graph, validator_node),
                    parameter_bindings: parameter_bindings.clone(),
                    needs_text_prebinding,
                }));
            }
        }
    }

    Ok(constraints)
}

pub fn parse_sparql_constraints<'a>(
    graph: &'a Graph,
    shape_node: NamedOrBlankNodeRef<'a>,
    is_property_shape: bool,
) -> Result<Vec<Constraint<'a>>, ShaclError> {
    let mut constraints = parse_direct_shape_sparql_constraints(graph, shape_node)?;
    constraints.extend(parse_component_sparql_constraints(
        graph,
        shape_node,
        is_property_shape,
    )?);
    Ok(constraints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ask_filter_is_not_flagged_as_unsupported() {
        // Regression test: spargebra >=0.4.6 wraps an ASK query's pattern in a
        // top-level `Project { variables: [], .. }` (0.4.5 didn't wrap it at all),
        // which used to false-positive here because the detection assumed ASK
        // queries could never have a top-level Project node.
        let query = r#"ASK {
            FILTER ($value != $requiredParam) .
        }"#;
        assert_eq!(unsupported_prebinding_construct(query, &[]), None);
    }

    #[test]
    fn plain_select_star_is_not_flagged_as_unsupported() {
        let query = "SELECT * WHERE { $this ?p ?o . }";
        assert_eq!(unsupported_prebinding_construct(query, &[]), None);
    }

    #[test]
    fn nested_select_not_reprojecting_this_is_flagged() {
        let query = r#"
            SELECT $this
            WHERE {
                $this ?x ?any .
                {
                    SELECT ?other ?b
                    WHERE {
                        ?other ?b ?c .
                    }
                }
            }"#;
        assert_eq!(
            unsupported_prebinding_construct(query, &[]),
            Some("Nested SELECT must explicitly project all pre-bound variables (e.g., $this)")
        );
    }

    #[test]
    fn nested_select_reprojecting_this_is_allowed() {
        let query = r#"
            SELECT $this
            WHERE {
                $this ?x ?any .
                {
                    SELECT $this ?b
                    WHERE {
                        $this ?b ?c .
                    }
                }
            }"#;
        assert_eq!(unsupported_prebinding_construct(query, &[]), None);
    }

    #[test]
    fn minus_is_flagged() {
        let query = "SELECT $this WHERE { $this ?p ?o . MINUS { $this a <urn:x> } }";
        assert_eq!(
            unsupported_prebinding_construct(query, &[]),
            Some("MINUS is not supported for SHACL pre-binding")
        );
    }

    #[test]
    fn service_is_flagged() {
        let query = "SELECT $this WHERE { SERVICE <http://example.org/sparql> { $this ?p ?o } }";
        assert_eq!(
            unsupported_prebinding_construct(query, &[]),
            Some("SERVICE is not supported for SHACL pre-binding")
        );
    }

    #[test]
    fn debug_pre_binding_003_needs_text_prebinding() {
        let query = r#"
            SELECT $this
            WHERE {
                {
                    {
                        FILTER ($this = <http://example.org/i>) .
                    }
                    FILTER bound($this) .
                }
                FILTER (true) .
            }"#;
        assert!(
            query_needs_text_prebinding(query, &[]),
            "003's bound($this) inside a nested group must trigger text pre-binding"
        );
    }

    #[test]
    fn debug_pre_binding_005_needs_text_prebinding() {
        let query = r#"
            SELECT $this ?code
            WHERE {
                {
                    FILTER (bound($this))
                }
                $this <http://example.org/property> "Label" .
                FILTER (bound($this)) .
            }"#;
        assert!(
            query_needs_text_prebinding(query, &[]),
            "005's bound($this) must trigger text pre-binding"
        );
    }

    #[test]
    fn debug_pre_binding_004_needs_text_prebinding() {
        let query = r#"
            SELECT $this
            WHERE {
                BIND ($this AS ?that) .
                FILTER (?that = <http://example.org/i>) .
            }"#;
        assert!(
            query_needs_text_prebinding(query, &[]),
            "004's BIND deriving from $this must trigger text pre-binding"
        );
    }

    #[test]
    fn debug_pre_binding_007_plain_nested_select_does_not_need_text_prebinding() {
        let query = r#"
            SELECT $this
            WHERE {
                {
                    SELECT $this
                    WHERE {
                        FILTER ($this = <http://example.org/i>) .
                    }
                }
            }"#;
        assert!(!query_needs_text_prebinding(query, &[]));
    }
}
