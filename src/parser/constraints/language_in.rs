use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::LanguageInConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait, sh, utils::parse_rdf_list, Constraint,
    ShaclError,
};

struct SHLanguageInConstraintParser;

impl ConstraintParserTrait for SHLanguageInConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        if let Some(language_in_node) =
            graph.object_for_subject_predicate(shape_node, sh::LANGUAGE_IN)
        {
            let language_in_node = match language_in_node {
                TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                _ => return Ok(vec![]),
            };

            let languages: Vec<String> = parse_rdf_list(graph, language_in_node)
                .into_iter()
                .filter_map(|term| match term {
                    TermRef::Literal(lit) => Some(lit.value().to_string()),
                    _ => None,
                })
                .collect();

            if !languages.is_empty() {
                return Ok(vec![Constraint::LanguageIn(LanguageInConstraint(
                    languages,
                ))]);
            }
        }
        Ok(vec![])
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHLanguageInConstraintParser
}
