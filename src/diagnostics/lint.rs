pub fn lint_shapes<'a>(
    _g: &'a oxigraph::model::Graph,
    _shapes: &'a [crate::Shape<'a>],
) -> Vec<super::Diagnostic> {
    Vec::new()
}
