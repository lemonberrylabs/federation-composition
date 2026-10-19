//! Port of `src/supergraph/validation/rules/satisfiablity-rule.ts`.

use graphql_js::GraphQLError;
use crate::supergraph::composition::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::JsResult;
use graphql_js::Value;
use super::satisfiablity::walker::WalkTracker;

/// `SatisfiabilityRule(context, supergraph)`. `new` builds the satisfiability `Supergraph` and runs `validate()`; fields: `error_by_coordinate` (`Record<string, WalkTracker[]>`; coordinates are type/field names, never integer-like).
/// Visitor callbacks: `InterfaceType`, `ObjectType`, `InterfaceTypeField`, `ObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct SatisfiabilityRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for SatisfiabilityRule<'s> {
    const NAME: &'static str = "SatisfiabilityRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        todo!()
    }
}

impl<'s> SatisfiabilityRule<'s> {
    /// `check(typeName, fieldName?)`: reports the unreachable paths recorded for the coordinate.
    fn check(&mut self, cx: &mut SupergraphValidationContext, type_name: &str, field_name: Option<&str>) -> JsResult<()> {
        todo!()
    }
}

impl<'s> SupergraphVisitor<'s> for SatisfiabilityRule<'s> {
    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        todo!()
    }

    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        todo!()
    }

    fn interface_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
        field_state: &'s InterfaceTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }

    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        todo!()
    }
}

/// An entry of `QueryPath`: `{ typeName, fieldName }` or `{ typeName }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPathEntry {
    pub type_name: String,
    pub field_name: Option<String>,
}

/// `QueryPath`.
pub type QueryPath = Vec<QueryPathEntry>;

fn print_line(msg: &str, indent_level: usize) -> String {
    todo!()
}

/// `printQueryPath(supergraphState, queryPath)`: the example operation in the error message.
fn print_query_path(supergraph_state: &SupergraphState, query_path: &QueryPath) -> JsResult<String> {
    todo!()
}

/// `createEmptyValueNode(fullType, supergraphState)`. Throws for unknown/unsupported types.
fn create_empty_value_node(full_type: &str, supergraph_state: &SupergraphState) -> JsResult<Value> {
    todo!()
}
