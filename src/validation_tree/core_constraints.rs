use oxigraph::model::NamedNode;

struct PathElement {
    is_inverse: bool,
    property: NamedNode,
}
