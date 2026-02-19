use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::DatatypeConstraint, parser::constraint_parser_trait::ConstraintParserTrait,
    sh, Constraint, ShaclError,
};

struct SHDatatypeConstraintParser;

impl ConstraintParserTrait for SHDatatypeConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        graph
            .object_for_subject_predicate(shape_node, sh::DATATYPE)
            .and_then(|t| match t {
                TermRef::NamedNode(nn) => Some(Constraint::Datatype(DatatypeConstraint(nn))),
                _ => None,
            })
            .into_iter()
            .map(Ok)
            .collect()
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHDatatypeConstraintParser
}
