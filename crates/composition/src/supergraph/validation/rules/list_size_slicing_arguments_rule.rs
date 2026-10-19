//! Port of `src/supergraph/validation/rules/list-size-slicing-arguments-rule.ts`.
//!
//! NOTE(port): the TS writes to the visited field states (`listSize.slicingArguments`,
//! `listSize.printRequireOneSlicingArgument`). The visitor only has shared references, so the
//! writes are recorded with `SupergraphValidationContext::defer_state_update` and applied after
//! the visit. Reading the old values during the visit gives the same results: the only reads
//! are of `slicingArguments.length` being non-zero, and a replacement list is never empty
//! (an empty one is an error instead of a write).

use graphql_js::GraphQLError;
use crate::supergraph::composition::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{
    SupergraphRule, SupergraphStateUpdate, SupergraphValidationContext,
};
use crate::utils::js::{JsError, JsResult};
use crate::subgraph::state::ListSize;

/// `ListSizeSlicingArgumentsRule(context, supergraph)`.
/// Visitor callbacks: `ObjectTypeField`, `InterfaceTypeField`.
pub struct ListSizeSlicingArgumentsRule<'s> {
    supergraph: &'s SupergraphState,
}

impl<'s> SupergraphRule<'s> for ListSizeSlicingArgumentsRule<'s> {
    const NAME: &'static str = "ListSizeSlicingArgumentsRule";

    fn new(context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        Ok(ListSizeSlicingArgumentsRule { supergraph })
    }
}

/// `listSize?.slicingArguments?.length` is truthy.
fn has_slicing_arguments(list_size: Option<&ListSize>) -> bool {
    list_size
        .and_then(|l| l.slicing_arguments.as_ref())
        .is_some_and(|a| !a.is_empty())
}

impl<'s> SupergraphVisitor<'s> for ListSizeSlicingArgumentsRule<'s> {
    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        let result = ensure_slicing_arguments_exist(
            &object_state.name,
            &field_state.name,
            field_state.list_size.as_ref(),
            |arg_name| field_state.args.get(arg_name).map(|a| a.by_graph.len()),
            field_state.by_graph.len(),
        )?;

        match result {
            SlicingArgumentsCheck::Error(error) => cx.report_error(error),
            SlicingArgumentsCheck::Replace(slicing_arguments) => {
                cx.defer_state_update(SupergraphStateUpdate::ObjectFieldSlicingArguments {
                    type_name: object_state.name.clone(),
                    field_name: field_state.name.clone(),
                    slicing_arguments,
                })
            }
            SlicingArgumentsCheck::Unchanged => {}
        }

        Ok(())
    }

    fn interface_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
        field_state: &'s InterfaceTypeFieldState,
    ) -> JsResult<()> {
        let result = ensure_slicing_arguments_exist(
            &interface_state.name,
            &field_state.name,
            field_state.list_size.as_ref(),
            |arg_name| field_state.args.get(arg_name).map(|a| a.by_graph.len()),
            field_state.by_graph.len(),
        )?;

        match result {
            SlicingArgumentsCheck::Error(error) => {
                cx.report_error(error);
                return Ok(());
            }
            SlicingArgumentsCheck::Replace(slicing_arguments) => {
                cx.defer_state_update(SupergraphStateUpdate::InterfaceFieldSlicingArguments {
                    type_name: interface_state.name.clone(),
                    field_name: field_state.name.clone(),
                    slicing_arguments,
                })
            }
            SlicingArgumentsCheck::Unchanged => {}
        }

        if !has_slicing_arguments(field_state.list_size.as_ref()) {
            return Ok(());
        }

        for implementor_name in &interface_state.implemented_by {
            let Some(implementor_type) = self.supergraph.object_types.get(implementor_name) else {
                continue;
            };

            let Some(implementor_field) = implementor_type.fields.get(&field_state.name) else {
                continue;
            };

            let Some(implementor_list_size) = implementor_field.list_size.as_ref() else {
                continue;
            };

            if !has_slicing_arguments(Some(implementor_list_size)) {
                continue;
            }

            // When interface field has slicing arguments, and the implementor field has slicing arguments,
            // we need to print the requireOneSlicingArgument argument in the supergraph SDL.
            // No idea why though, I would print it only when they are different or something.
            // It's `true` by default anyway, so it doesn't really matter.
            cx.defer_state_update(SupergraphStateUpdate::InterfaceFieldPrintRequireOneSlicingArgument {
                type_name: interface_state.name.clone(),
                field_name: field_state.name.clone(),
            });
        }

        Ok(())
    }
}

/// The outcome of `ensureSlicingArgumentsExist`.
#[derive(Debug)]
enum SlicingArgumentsCheck {
    /// Returns `undefined` without writing.
    Unchanged,
    /// Returns the error.
    Error(GraphQLError),
    /// Returns `undefined` after `fieldState.listSize.slicingArguments = newslicingArguments`.
    Replace(Vec<String>),
}

/// `ensureSlicingArgumentsExist(typeName, fieldState)` for an object or interface field: its
/// `name`, `listSize`, the `byGraph.size` of an argument by name (`None` when the field has no
/// such argument) and its own `byGraph.size`. Throws when a slicing argument is not an
/// argument of the field.
fn ensure_slicing_arguments_exist(
    type_name: &str,
    field_name: &str,
    list_size: Option<&ListSize>,
    arg_by_graph_size: impl Fn(&str) -> Option<usize>,
    field_by_graph_size: usize,
) -> JsResult<SlicingArgumentsCheck> {
    let Some(slicing_arguments) = list_size
        .and_then(|l| l.slicing_arguments.as_ref())
        .filter(|a| !a.is_empty())
    else {
        return Ok(SlicingArgumentsCheck::Unchanged);
    };

    let mut newslicing_arguments: Vec<String> = Vec::new();
    for arg_name in slicing_arguments {
        let Some(arg_size) = arg_by_graph_size(arg_name) else {
            return Err(JsError::error(
                "Could not find the argument in the field annotated with @listSize",
            ));
        };

        if arg_size != field_by_graph_size {
            continue;
        }

        newslicing_arguments.push(arg_name.clone());
    }

    if newslicing_arguments.is_empty() {
        return Ok(SlicingArgumentsCheck::Error(GraphQLError::with_code(
            [
                format!(
                    "All arguments for @listSize(slicingArguments:) on field \"{type_name}.{field_name}\" were disregarded."
                ),
                "For an argument to be valid, it must be defined in every subgraph where the field exists.".to_string(),
            ]
            .join(" "),
            "LIST_SIZE_INVALID_SLICING_ARGUMENT",
        )));
    }

    Ok(SlicingArgumentsCheck::Replace(newslicing_arguments))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_size(slicing: Option<&[&str]>) -> ListSize {
        ListSize {
            print_require_one_slicing_argument: false,
            assumed_size: None,
            slicing_arguments: slicing.map(|s| s.iter().map(|x| x.to_string()).collect()),
            sized_fields: None,
            require_one_slicing_argument: true,
        }
    }

    #[test]
    fn no_slicing_arguments_is_unchanged() {
        let ls = list_size(Some(&[]));
        let r = ensure_slicing_arguments_exist("T", "f", Some(&ls), |_| Some(1), 1).unwrap();
        assert!(matches!(r, SlicingArgumentsCheck::Unchanged));
        let r = ensure_slicing_arguments_exist("T", "f", None, |_| Some(1), 1).unwrap();
        assert!(matches!(r, SlicingArgumentsCheck::Unchanged));
    }

    #[test]
    fn keeps_arguments_defined_in_every_graph_of_the_field() {
        let ls = list_size(Some(&["first", "last"]));
        let r = ensure_slicing_arguments_exist(
            "T",
            "f",
            Some(&ls),
            |a| Some(if a == "first" { 2 } else { 1 }),
            2,
        )
        .unwrap();
        match r {
            SlicingArgumentsCheck::Replace(args) => assert_eq!(args, vec!["first".to_string()]),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reports_when_every_argument_is_disregarded() {
        let ls = list_size(Some(&["first"]));
        let r = ensure_slicing_arguments_exist("T", "f", Some(&ls), |_| Some(1), 2).unwrap();
        match r {
            SlicingArgumentsCheck::Error(e) => {
                assert_eq!(
                    e.message,
                    "All arguments for @listSize(slicingArguments:) on field \"T.f\" were disregarded. For an argument to be valid, it must be defined in every subgraph where the field exists."
                );
                assert_eq!(e.code(), Some("LIST_SIZE_INVALID_SLICING_ARGUMENT"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn throws_for_an_unknown_argument() {
        let ls = list_size(Some(&["first"]));
        let r = ensure_slicing_arguments_exist("T", "f", Some(&ls), |_| None, 1);
        assert_eq!(
            r.unwrap_err().to_js_string(),
            "Error: Could not find the argument in the field annotated with @listSize"
        );
    }
}
