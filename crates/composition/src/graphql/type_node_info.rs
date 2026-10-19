//! Port of `src/graphql/type-node-info.ts`.
//!
//! `TypeNodeInfo` tracks the type definition/extension (or directive definition), field,
//! argument and enum value being visited. It keeps references into the visited document.

use graphql_js::{EnumValueDefinition, InputValueDefinition, NodeRef, VisitAction, VisitInfo, Visitor};

/// `TypeNodeInfo`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeNodeInfo<'a> {
    /// A `TypeDefinition`, `TypeExtension` or `DirectiveDefinition` node.
    type_: Option<NodeRef<'a>>,
    /// A `FieldDefinition` or (for input objects and directive definitions) an
    /// `InputValueDefinition`.
    field: Option<NodeRef<'a>>,
    arg: Option<&'a InputValueDefinition>,
    value: Option<&'a EnumValueDefinition>,
}

impl<'a> TypeNodeInfo<'a> {
    pub fn new() -> Self {
        TypeNodeInfo::default()
    }

    /// `getTypeDef()`.
    pub fn get_type_def(&self) -> Option<NodeRef<'a>> {
        self.type_
    }

    /// `getFieldDef()`.
    pub fn get_field_def(&self) -> Option<NodeRef<'a>> {
        self.field
    }

    /// `getArgumentDef()`.
    pub fn get_argument_def(&self) -> Option<&'a InputValueDefinition> {
        self.arg
    }

    /// `getValueDef()`.
    pub fn get_value_def(&self) -> Option<&'a EnumValueDefinition> {
        self.value
    }

    pub fn enter(&mut self, node: NodeRef<'a>) {
        todo!()
    }

    pub fn leave(&mut self, node: NodeRef<'a>) {
        todo!()
    }
}

/// A visit context that owns the `TypeNodeInfo` maintained by [`visit_with_type_node_info`].
/// In TS the `TypeNodeInfo` instance is shared by the wrapper and the visitors through
/// closures; here it lives in the context every visitor receives.
pub trait HasTypeNodeInfo<'a> {
    fn type_node_info_mut(&mut self) -> &mut TypeNodeInfo<'a>;

    /// Whether a visitor recorded a thrown error (the port of a TS `throw` inside a visitor
    /// callback, which aborts the whole `visit`). The wrapper then returns
    /// `VisitAction::Break` so the traversal stops for every parallel visitor.
    fn has_thrown(&self) -> bool {
        false
    }
}

/// The visitor returned by `visitWithTypeNodeInfo`.
pub struct VisitWithTypeNodeInfo<V> {
    visitor: V,
}

/// `visitWithTypeNodeInfo(typeInfo, visitor)`: a visitor that maintains the context's
/// `TypeNodeInfo` along with `visitor`. When the inner `enter` returns something other than
/// `Continue`, the TS calls `typeInfo.leave(node)` right away (graphql-js then skips `leave`).
pub fn visit_with_type_node_info<V>(visitor: V) -> VisitWithTypeNodeInfo<V> {
    VisitWithTypeNodeInfo { visitor }
}

impl<'a, C: HasTypeNodeInfo<'a>, V: Visitor<'a, C>> Visitor<'a, C> for VisitWithTypeNodeInfo<V> {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }

    fn leave(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        todo!()
    }
}
