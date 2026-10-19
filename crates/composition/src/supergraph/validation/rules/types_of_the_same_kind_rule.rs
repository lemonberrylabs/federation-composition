//! Port of `src/supergraph/validation/rules/types-of-the-same-kind-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::validation::validation_context::SupergraphValidationContext;
use indexmap::{IndexMap, IndexSet};
use crate::subgraph::state::TypeKind;

/// `TypesOfTheSameKindRule(context)`: a pre-supergraph rule (runs before the subgraphs are merged).
pub fn types_of_the_same_kind_rule(context: &mut SupergraphValidationContext) {
    todo!()
}

/// `mapIRKindToString[kind]`.
fn map_ir_kind_to_string(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Object => "Object",
        TypeKind::Interface => "Interface",
        TypeKind::Union => "Union",
        TypeKind::Enum => "Enum",
        TypeKind::InputObject => "InputObject",
        TypeKind::Scalar => "Scalar",
        TypeKind::Directive => "Directive",
    }
}

/// `GraphTypeValidationContext`.
/// NOTE(port): the TS keeps these objects in a `Set`, which dedupes by identity (never here,
/// as each is a fresh object); use a `Vec`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphTypeValidationContext {
    pub graph_name: String,
    pub is_interface_object: bool,
}

fn interface_object_conditions(kind_to_graphs: &IndexMap<TypeKind, Vec<GraphTypeValidationContext>>) -> bool {
    todo!()
}
