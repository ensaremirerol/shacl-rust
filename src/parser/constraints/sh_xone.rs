use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::XoneConstraint, parser::constraint_parser_trait::ConstraintParserTrait, sh,
    utils::parse_rdf_list, Constraint, ShaclError,
};

struct SHXoneConstraintParser;

impl ConstraintParserTrait for SHXoneConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        let mut constraints = Vec::new();

        for xone_obj in graph.objects_for_subject_predicate(shape_node, sh::XONE) {
            let xone_node = match xone_obj {
                TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                _ => continue,
            };

            let shape_refs = parse_rdf_list(graph, xone_node);
            let mut xone_shapes = Vec::new();
            for shape_ref in shape_refs {
                let sn = match shape_ref {
                    TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                    TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                    _ => continue,
                };
                if let Ok(sub_shape) = super::super::parse_shape(graph, sn, Some(shape_node)) {
                    xone_shapes.push(sub_shape);
                }
            }

            if !xone_shapes.is_empty() {
                constraints.push(Constraint::Xone(XoneConstraint(xone_shapes)));
            }
        }

        Ok(constraints)
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHXoneConstraintParser
}
