//! Port of `src/supergraph/validation/rules/satisfiablity/selection.ts`.

use std::cell::RefCell;
use std::rc::Rc;

use graphql_js::{Field as FieldNode, InlineFragment, SelectionSet};
use indexmap::IndexMap;

use crate::supergraph::state::SupergraphState;
use crate::utils::js::JsResult;

/// `Field` (`kind: "field"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub type_name: String,
    pub field_name: String,
    pub selection_set: Option<Vec<SelectionNode>>,
}

/// `Fragment` (`kind: "fragment"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub type_name: String,
    pub selection_set: Vec<SelectionNode>,
}

/// `SelectionNode = Field | Fragment`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionNode {
    Field(Field),
    Fragment(Fragment),
}

impl SelectionNode {
    pub fn type_name(&self) -> &str {
        match self {
            SelectionNode::Field(f) => &f.type_name,
            SelectionNode::Fragment(f) => &f.type_name,
        }
    }

    /// `selectionSet` (`null` for a leaf field).
    pub fn selection_set(&self) -> Option<&[SelectionNode]> {
        match self {
            SelectionNode::Field(f) => f.selection_set.as_deref(),
            SelectionNode::Fragment(f) => Some(&f.selection_set),
        }
    }
}

/// `Selection`: resolved key/requires/provides fields. Shared (`Rc<Selection>`) between the
/// resolver cache and the moves, as the TS shares the object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    type_name: String,
    source: String,
    pub selection_set: Vec<SelectionNode>,
}

impl Selection {
    pub fn new(type_name: String, source: String, selection_set: Vec<SelectionNode>) -> Self {
        Selection {
            type_name,
            source,
            selection_set,
        }
    }

    pub fn contains(&self, type_name: &str, field_name: &str) -> bool {
        self.contains_in(type_name, field_name, &self.selection_set)
    }

    pub fn equals(&self, other: &Selection) -> bool {
        todo!()
    }

    fn selection_set_equal(&self, selection_set: &[SelectionNode], other_selection_set: &[SelectionNode]) -> bool {
        todo!()
    }

    /// `_contains(typeName, fieldName, selectionSet)`.
    fn contains_in(&self, type_name: &str, field_name: &str, selection_set: &[SelectionNode]) -> bool {
        todo!()
    }

    /// `toString()`: the source with whitespace runs collapsed (JS `\s+`).
    pub fn to_string(&self) -> String {
        todo!()
    }
}

/// `SelectionResolver`: parses and resolves field sets, with a cache keyed by
/// `typeName/keyFields`. Shared as `Rc<SelectionResolver>` by all graphs of a `Supergraph`.
#[derive(Debug)]
pub struct SelectionResolver<'s> {
    cache: RefCell<IndexMap<String, Rc<Selection>>>,
    supergraph_state: &'s SupergraphState,
}

impl<'s> SelectionResolver<'s> {
    pub fn new(supergraph_state: &'s SupergraphState) -> Self {
        SelectionResolver {
            cache: RefCell::new(IndexMap::new()),
            supergraph_state,
        }
    }

    /// `resolve(typeName, keyFields)`. Throws for an unknown type or unparsable fields.
    pub fn resolve(&self, type_name: &str, key_fields: &str) -> JsResult<Rc<Selection>> {
        todo!()
    }

    fn key_factory(&self, type_name: &str, key_fields: &str) -> String {
        format!("{type_name}/{key_fields}")
    }

    fn resolve_field_node(&self, type_name: &str, field_node: &FieldNode, selection_set: &mut Vec<SelectionNode>) -> JsResult<()> {
        todo!()
    }

    fn resolve_inline_fragment_node(
        &self,
        type_name: &str,
        fragment_node: &InlineFragment,
        selection_set: &mut Vec<SelectionNode>,
    ) -> JsResult<()> {
        todo!()
    }

    fn resolve_selection_set_node(&self, type_name: &str, selection_set_node: &SelectionSet) -> JsResult<Vec<SelectionNode>> {
        todo!()
    }

    /// `sort(selectionSet)`: the comparator of the TS (by kind, type name, field name).
    fn sort(&self, selection_set: Vec<SelectionNode>) -> Vec<SelectionNode> {
        todo!()
    }
}
