#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

// A port of graphql-js 17.0.2 by the GraphQL Contributors, under the MIT license. The module
// tree mirrors graphql-js's files; the modules are private and the public API is re-exported
// below. What the rest of this repository needs beyond it is in `__internal`.

mod ast;
mod block_string;
pub mod collections;
mod directive_location;
mod error;
mod execution;
mod js;
mod jsutils;
mod lexer;
mod nesting;
mod parser;
mod predicates;
mod printer;
mod specified;
mod r#type;
mod utilities;
mod validation;
mod visitor;

pub use ast::*;
pub use directive_location::DirectiveLocation;
pub use error::GraphQLError;
pub use nesting::check_nesting_depth;
pub use parser::{
    parse, parse_const_value, parse_schema_coordinate, parse_type, parse_value, parse_with_options, ParseOptions,
    MAX_NESTING_DEPTH,
};
pub use predicates::*;
pub use printer::print;
pub use r#type::definition::{
    GraphQLArgument, GraphQLEnumType, GraphQLEnumValue, GraphQLField, GraphQLInputField, GraphQLInputObjectType,
    GraphQLInterfaceType, GraphQLNamedType, GraphQLObjectType, GraphQLScalarType, GraphQLType, GraphQLUnionType,
};
pub use r#type::directives::GraphQLDirective;
pub use r#type::schema::{GraphQLSchema, Implementations};
pub use r#type::validate::{assert_valid_schema, validate_schema};
pub use specified::{
    is_introspection_type, is_specified_directive, is_specified_scalar_type, specified_directives,
    specified_scalar_type, specified_scalar_types, CoercedLiteral, SpecifiedScalarType, VersionInfo,
    DEFAULT_DEPRECATION_REASON, GRAPHQL_MAX_INT, GRAPHQL_MIN_INT, INTROSPECTION_TYPE_NAMES, VERSION, VERSION_INFO,
};
pub use utilities::build_ast_schema::{build_ast_schema, build_schema};
pub use utilities::{concat_ast, strip_ignored_characters};
pub use validation::validate::{assert_valid_sdl, validate_sdl};
pub use visitor::{
    visit, visit_edit, visit_in_parallel, visit_in_parallel_edit, Ancestor, EditAction, EditResult, EditVisitor,
    KindSet, ParallelEditVisitor, ParallelVisitor, PathKey, VisitAction, VisitInfo, VisitKinds, Visitor,
};

/// Internals for this repository's differential tests. **Not part of the public API**:
/// anything here can change or disappear in any release.
#[doc(hidden)]
pub mod __internal {
    /// graphql-js `jsutils`.
    pub mod jsutils {
        pub use crate::jsutils::JsValue;
    }

    /// graphql-js `type`.
    pub mod r#type {
        /// graphql-js `type/definition`: what the differential tests compare beyond the public
        /// API, the defaults as graphql-js keeps them (a literal, or a value given in code).
        pub mod definition {
            pub use crate::r#type::definition::DefaultInput;

            /// The argument's graphql-js `default`.
            pub fn argument_default(arg: &crate::GraphQLArgument) -> Option<&DefaultInput> {
                arg.default.as_ref()
            }

            /// The input field's graphql-js `default`.
            pub fn input_field_default(field: &crate::GraphQLInputField) -> Option<&DefaultInput> {
                field.default.as_ref()
            }
        }
    }
}
