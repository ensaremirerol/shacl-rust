use oxigraph::model::{vocab::rdf::TYPE, TermRef};

use crate::{
    core::{constraints::ClassConstraint, path::Path, shape::Shape},
    utils,
    validation::{dataset::ValidationDataset, Validate, ValidationResult, ViolationBuilder},
    vocab::sh,
    ShaclError,
};

impl<'a> Validate<'a> for ClassConstraint<'a> {
    fn validate(
        &'a self,
        validation_dataset: &'a ValidationDataset,
        focus_node: TermRef<'a>,
        _path: Option<&'a Path<'a>>,
        value_nodes: &[TermRef<'a>],
        shape: &'a Shape<'a>,
    ) -> Result<Vec<ValidationResult<'a>>, ShaclError> {
        let mut violations = Vec::new();
        let data_graph = validation_dataset.data_graph();

        // sh:class matches SHACL instances: the class itself plus everything
        // reachable via rdfs:subClassOf, resolved once for all value nodes.
        let accepted_classes = utils::collect_all_subclasses(self.0.into(), data_graph);

        for &value_node in value_nodes {
            if let Some(value_as_node) = utils::term_to_named_or_blank(value_node) {
                let is_instance = data_graph
                    .objects_for_subject_predicate(value_as_node, TYPE)
                    .any(|object| match object {
                        TermRef::NamedNode(class) => accepted_classes.contains(&class),
                        _ => false,
                    });

                if !is_instance {
                    let builder = ViolationBuilder::new(focus_node)
                        .value(value_node)
                        .message(format!("Value is not an instance of class {}", self.0))
                        .component(sh::CLASS_CONSTRAINT_COMPONENT)
                        .detail(format!("sh:class {}", self.0));

                    violations.push(shape.build_validation_result(builder));
                }
            } else {
                let builder = ViolationBuilder::new(focus_node)
                    .value(value_node)
                    .message("Value must be a node to check class membership")
                    .component(sh::CLASS_CONSTRAINT_COMPONENT)
                    .detail(format!("sh:class {}", self.0));

                violations.push(shape.build_validation_result(builder));
            }
        }

        Ok(violations)
    }
}
