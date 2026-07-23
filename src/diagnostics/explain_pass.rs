pub fn explain_conformance<'a>(
    _d: &'a crate::validation::dataset::ValidationDataset,
    _s: &'a [crate::Shape<'a>],
    _f: oxigraph::model::TermRef<'a>,
    _sh: Option<oxigraph::model::NamedOrBlankNodeRef<'a>>,
) -> Vec<super::Diagnostic> {
    Vec::new()
}
