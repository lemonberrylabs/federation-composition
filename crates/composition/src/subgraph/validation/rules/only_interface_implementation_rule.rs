//! Port of `src/subgraph/validation/rules/only-interface-implementation-rule.ts`.

use graphql_js::{Definition, GraphQLError, NodeRef, VisitAction, VisitInfo, Visitor};
use indexmap::IndexMap;

use crate::subgraph::validation::validation_context::{SimpleRule, SimpleValidationContext};
use graphql_js::NamedType;

/// `OnlyInterfaceImplementationRule(context)`.
/// Visitor callbacks: `ObjectTypeDefinition: check`, `ObjectTypeExtension: check`.
/// Fields: the locals of the TS factory.
pub struct OnlyInterfaceImplementationRule {
    filled: bool,
    /// `"ObjectType" | "InterfaceType" | "UnionType" | "EnumType" | "InputObjectType" | "ScalarType"`.
    type_name_to_kind: IndexMap<String, &'static str>,
}

impl<'a> SimpleRule<'a> for OnlyInterfaceImplementationRule {
    const NAME: &'static str = "OnlyInterfaceImplementationRule";

    fn new(context: &mut SimpleValidationContext<'a>) -> Self {
        OnlyInterfaceImplementationRule {
            filled: false,
            type_name_to_kind: IndexMap::new(),
        }
    }
}

impl OnlyInterfaceImplementationRule {
    /// `fillTypeNameToKindMap()` (lazily, on the first `check`).
    fn fill_type_name_to_kind_map(&mut self, context: &SimpleValidationContext<'_>) {
        for node in &context.get_document().definitions {
            let (name, kind) = match node {
                Definition::ObjectTypeDefinition(n) => (&n.name, "ObjectType"),
                Definition::ObjectTypeExtension(n) => (&n.name, "ObjectType"),
                Definition::InterfaceTypeDefinition(n) => (&n.name, "InterfaceType"),
                Definition::InterfaceTypeExtension(n) => (&n.name, "InterfaceType"),
                Definition::UnionTypeDefinition(n) => (&n.name, "UnionType"),
                Definition::UnionTypeExtension(n) => (&n.name, "UnionType"),
                Definition::EnumTypeDefinition(n) => (&n.name, "EnumType"),
                Definition::EnumTypeExtension(n) => (&n.name, "EnumType"),
                Definition::ScalarTypeDefinition(n) => (&n.name, "ScalarType"),
                Definition::ScalarTypeExtension(n) => (&n.name, "ScalarType"),
                Definition::InputObjectTypeDefinition(n) => (&n.name, "InputObjectType"),
                Definition::InputObjectTypeExtension(n) => (&n.name, "InputObjectType"),
                _ => continue,
            };
            self.type_name_to_kind.insert(name.value.clone(), kind);
        }

        self.filled = true;
    }

    /// `findKindByName(typeName)`.
    fn find_kind_by_name(&mut self, context: &SimpleValidationContext<'_>, type_name: &str) -> Option<&'static str> {
        if !self.filled {
            self.fill_type_name_to_kind_map(context);
        }

        self.type_name_to_kind.get(type_name).copied()
    }

    /// `check(node)` for an object type definition or extension (its `name` and `interfaces`).
    fn check(&mut self, context: &mut SimpleValidationContext<'_>, name: &str, interfaces: &[NamedType]) {
        // `if (!node.interfaces) return;` is the same as iterating an empty list.
        for interface_node in interfaces {
            let interface_name = &interface_node.name.value;

            let kind = self.find_kind_by_name(context, interface_name);

            if let Some(kind) = kind {
                if kind != "InterfaceType" {
                    context.report_error(GraphQLError::with_code(
                        format!("Cannot implement non-interface type {interface_name} (of type ObjectType)"),
                        "INVALID_GRAPHQL",
                    ));
                }
            }
        }
    }
}

impl<'a> Visitor<'a, SimpleValidationContext<'a>> for OnlyInterfaceImplementationRule {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut SimpleValidationContext<'a>, info: &VisitInfo<'a, '_>) -> VisitAction {
        match node {
            NodeRef::ObjectTypeDefinition(node) => self.check(cx, &node.name.value, &node.interfaces),
            NodeRef::ObjectTypeExtension(node) => self.check(cx, &node.name.value, &node.interfaces),
            _ => {}
        }
        VisitAction::Continue
    }
}
