pub fn from_report<'a>(
    _report: &crate::ValidationReport<'a>,
    _dataset: &'a crate::validation::dataset::ValidationDataset,
    _shapes: &'a [crate::Shape<'a>],
) -> Vec<super::Diagnostic> {
    Vec::new()
}
