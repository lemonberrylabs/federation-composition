//! Port of graphql-js `LoneSchemaDefinitionRule`.

use crate::ast::{Kind, NodeRef};
use crate::error::GraphQLError;
use crate::validation::validation_context::SDLValidationContext;
use crate::visitor::{VisitAction, VisitInfo, VisitKinds, Visitor};

/// graphql-js `LoneSchemaDefinitionRule`: a document defines the schema at
/// most once.
pub struct LoneSchemaDefinitionRule {
    schema_definitions_count: usize,
}

impl LoneSchemaDefinitionRule {
    pub fn new(_context: &mut SDLValidationContext<'_>) -> Self {
        // NOTE(port): `alreadyDefined` is about the schema being extended;
        // there is none.
        LoneSchemaDefinitionRule {
            schema_definitions_count: 0,
        }
    }
}

impl<'a> Visitor<'a, SDLValidationContext<'a>> for LoneSchemaDefinitionRule {
    fn kinds(&self) -> VisitKinds {
        VisitKinds::enter(&[Kind::SchemaDefinition])
    }

    fn enter(
        &mut self,
        node: NodeRef<'a>,
        context: &mut SDLValidationContext<'a>,
        _info: &VisitInfo<'a, '_>,
    ) -> VisitAction {
        if let NodeRef::SchemaDefinition(_) = node {
            if self.schema_definitions_count > 0 {
                context.report_error(GraphQLError::new(
                    "Must provide only one schema definition.",
                ));
            }
            self.schema_definitions_count += 1;
        }
        VisitAction::Continue
    }
}
