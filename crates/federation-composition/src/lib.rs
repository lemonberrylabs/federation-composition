#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

// Port of `@theguild/federation-composition` (`src/`) by The Guild (<https://the-guild.dev>),
// under its MIT license. See `crates/PORTING.md`. The module tree mirrors upstream's `src/` one
// to one; the modules are private, and `src/index.ts`'s export list is re-exported below with
// Rust naming. What the tests and the WebAssembly bindings need beyond that is in `__internal`.
//
// The `graphql-js` port is the `graphql_core` crate; `crate::graphql` is `src/graphql/`.

mod compose;
mod contracts;
mod error;
mod graphql;
mod specifications;
mod subgraph;
mod supergraph;
mod types;
mod utils;
mod validate;

pub use graphql_core;
pub use graphql_core::GraphQLError;

// The repository README's Rust examples, run as doctests so they keep compiling against this API.
// Only under `cfg(doctest)`: the packaged crate does not contain the repository README.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct RepositoryReadme;

pub use error::Error;

// `src/index.ts`: `export * from "./compose.js"`.
pub use compose::{
    composition_has_errors, create_composition_success_read_cache_container, CompositionFailure, CompositionResult,
    CompositionSuccess,
};

/// Composes subgraphs into a supergraph: upstream's `composeServices(services)`.
///
/// The subgraphs are validated first; invalid or incompatible subgraphs give a
/// [`CompositionResult::Failure`] with upstream's errors (message and `extensions.code`, in
/// upstream's order). On success the supergraph SDL is byte for byte what upstream produces.
///
/// `Err` where upstream throws instead of returning a failure, for example for two subgraphs
/// with the same name.
///
/// ```
/// use federation_composition::{compose_services, graphql_core, ServiceDefinition};
///
/// let users = ServiceDefinition::new(
///     "users",
///     graphql_core::parse(
///         r#"
///         extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])
///         type Query { me: User }
///         type User @key(fields: "id") { id: ID! name: String }
///         "#,
///     )?,
/// );
/// let result = compose_services(&[users])?;
/// assert!(result.is_success());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn compose_services(services: &[ServiceDefinition]) -> Result<CompositionResult, Error> {
    check_services(services)?;
    Ok(compose::compose_services(services, None)?)
}

/// Upstream's `assertCompositionSuccess(compositionResult, message?)`: `Err` with `message`
/// (default `"Unexpected test failure"`) when the composition failed.
pub fn assert_composition_success(composition_result: &CompositionResult, message: Option<&str>) -> Result<(), Error> {
    Ok(compose::assert_composition_success(composition_result, message)?)
}

/// Upstream's `assertCompositionFailure(compositionResult, message?)`: `Err` with `message`
/// (default `"Unexpected test failure"`) when the composition succeeded.
pub fn assert_composition_failure(composition_result: &CompositionResult, message: Option<&str>) -> Result<(), Error> {
    Ok(compose::assert_composition_failure(composition_result, message)?)
}

// `export * from "./types.js"`.
pub use types::ServiceDefinition;

// `export * from "./validate.js"`.
pub use validate::{ValidationResult, ValidationSuccess};

/// Validates subgraphs without building the supergraph document: upstream's
/// `validate(subgraphs)`, the first step of [`compose_services`].
///
/// `Err` where upstream throws, as for [`compose_services`].
pub fn validate(subgraphs: &[ServiceDefinition]) -> Result<ValidationResult, Error> {
    check_services(subgraphs)?;
    Ok(validate::validate(subgraphs, None)?)
}

/// Validates one subgraph on its own: upstream's `validateSubgraph(subgraph)`. The errors are
/// the subgraph-level validation errors (an empty list for a valid subgraph); the checks that
/// need all subgraphs together are not run.
///
/// `Err` where upstream throws.
pub fn validate_subgraph(subgraph: &ServiceDefinition) -> Result<Vec<GraphQLError>, Error> {
    check_nesting_depth(&subgraph.type_defs)?;
    Ok(validate::validate_subgraph(subgraph)?)
}

// `export * from "./utils/link/index.js"`, with the types its functions return.
pub use utils::link::link_url::SupportsVersion;
pub use utils::link::{
    FederatedLink, FederatedLinkImport, FederatedLinkUrl, LinkImplementations, LinkVersion, FEDERATION_V1,
};

/// The `@link`s of a schema and the names their imports resolve to: upstream's
/// `extractLinkImplementations(typeDefs)`, for implementing `@link` features of your own.
///
/// `Err` where upstream throws: an invalid `url` or `import` argument of a `@link`.
///
/// ```
/// use federation_composition::{extract_link_implementations, graphql_core, LinkVersion};
///
/// let type_defs = graphql_core::parse(
///     r#"
///     extend schema
///       @link(url: "https://specs.graphql-hive.com/metadata/v0.1", import: [{ name: "@meta", as: "@m" }])
///     "#,
/// )?;
/// let links = extract_link_implementations(&type_defs)?;
/// let metadata = "https://specs.graphql-hive.com/metadata";
/// assert!(links.matches_implementation(metadata, &LinkVersion::Str("v0.1".into())));
/// assert!(!links.matches_implementation(metadata, &LinkVersion::Str("v0.2".into())));
/// // An imported name resolves to its alias, any other name to the namespaced one.
/// assert_eq!(links.resolve_import_name(metadata, "@meta"), "m");
/// assert_eq!(links.resolve_import_name(metadata, "@other"), "metadata__other");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_link_implementations(type_defs: &graphql_core::Document) -> Result<LinkImplementations, Error> {
    check_nesting_depth(type_defs)?;
    utils::link::extract_link_implementations(type_defs)
}

/// The public schema of a supergraph: upstream's `transformSupergraphToPublicSchema`. Removes
/// the federation directives and types and the `@inaccessible` elements.
///
/// `Err` where upstream throws (an invalid `@link` in the supergraph).
pub fn transform_supergraph_to_public_schema(
    supergraph: &graphql_core::Document,
) -> Result<graphql_core::Document, Error> {
    check_nesting_depth(supergraph)?;
    Ok(graphql::transform_supergraph_to_public_schema::transform_supergraph_to_public_schema(supergraph)?)
}

pub use graphql::contains_supergraph_spec::contains_supergraph_spec;

/// A copy of the document with definitions, fields, arguments, values and directives sorted:
/// upstream's `sortSDL(doc)`, which makes two equivalent schemas print the same.
///
/// `Err` where upstream throws: a syntax error in the `fields` of a `@requires` or
/// `@provides`.
pub fn sort_sdl(doc: &graphql_core::Document) -> Result<graphql_core::Document, Error> {
    check_nesting_depth(doc)?;
    Ok(graphql::sort_sdl::sort_sdl(doc)?)
}

pub use contracts::tag_extraction::SchemaContractFilter;

/// Composes a schema contract: upstream's `composeSchemaContract(services, tagFilter,
/// removeUnreachableTypes)`. The subgraphs are filtered with `tag_filter`
/// ([`apply_tag_filter_on_subgraphs`]) and composed; with `remove_unreachable_types` (upstream's
/// default is `true`), types the public schema cannot reach become `@inaccessible`.
///
/// `Err` where upstream throws.
pub fn compose_schema_contract(
    services: &[ServiceDefinition],
    tag_filter: &SchemaContractFilter,
    remove_unreachable_types: bool,
) -> Result<CompositionResult, Error> {
    check_services(services)?;
    Ok(contracts::schema_contract::compose_schema_contract(
        services,
        tag_filter,
        remove_unreachable_types,
    )?)
}

/// Upstream's `addInaccessibleToUnreachableTypes(resolveName, compositionResult)`: a new
/// success whose supergraph has `@inaccessible` on every type the public schema cannot reach,
/// except the federation types. `resolve_name(identity, name)` resolves spec names, as
/// [`LinkImplementations::resolve_import_name`] does for the supergraph.
///
/// `Err` where upstream throws (building the public schema).
pub fn add_inaccessible_to_unreachable_types(
    resolve_name: &dyn Fn(&str, &str) -> String,
    composition_result: &CompositionSuccess,
) -> Result<CompositionSuccess, Error> {
    check_nesting_depth(composition_result.supergraph_document_node())?;
    Ok(
        contracts::add_inaccessible_to_unreachable_types::add_inaccessible_to_unreachable_types(
            resolve_name,
            composition_result,
        )?,
    )
}

/// Upstream's `applyTagFilterOnSubgraphs(subgraphs, filter)`: the subgraphs with `@inaccessible`
/// on what the tag filter removes from a contract.
///
/// `Err` where upstream throws.
pub fn apply_tag_filter_on_subgraphs(
    subgraphs: &[ServiceDefinition],
    filter: &SchemaContractFilter,
) -> Result<Vec<ServiceDefinition>, Error> {
    check_services(subgraphs)?;
    Ok(contracts::tag_extraction::apply_tag_filter_on_subgraphs(
        subgraphs, filter,
    )?)
}

/// Not in upstream: the functions above that take documents reject one that nests deeper than
/// the parser allows ([`graphql_core::MAX_NESTING_DEPTH`]), which only a document built in code or
/// deserialized can, before they recurse into it (see SECURITY.md, "Input limits"). The `Err` is
/// a [`GraphQLError`] with the message `AST nesting depth exceeds 128.`
pub(crate) fn check_nesting_depth(document: &graphql_core::Document) -> Result<(), Error> {
    graphql_core::check_nesting_depth(document, graphql_core::MAX_NESTING_DEPTH)
        .map_err(|error| Error::from(utils::js::JsError::GraphQL(error)))
}

/// [`check_nesting_depth`] for each subgraph's type definitions.
fn check_services(services: &[ServiceDefinition]) -> Result<(), Error> {
    services
        .iter()
        .try_for_each(|service| check_nesting_depth(&service.type_defs))
}

/// Internals for this repository's test suite and WebAssembly bindings. **Not part of the
/// public API**: anything here can change or disappear in any release.
///
/// The submodules mirror upstream's `src/` paths (`__internal::compose` is `src/compose.ts`),
/// and their functions keep upstream's signatures, including the `__internal` options and the
/// JavaScript error type (`utils::js::JsError`).
#[doc(hidden)]
pub mod __internal {
    /// `src/compose.ts`.
    pub mod compose {
        pub use crate::compose::{assert_composition_failure, assert_composition_success, compose_services};
    }

    /// `src/contracts/`.
    pub mod contracts {
        /// `src/contracts/reachable-type-filter.ts`.
        pub mod reachable_type_filter {
            pub use crate::contracts::reachable_type_filter::{
                add_directive_on_types, get_reachable_types, AddDirectiveOnTypesArgs,
            };
        }

        /// `src/contracts/tag-extraction.ts`.
        pub mod tag_extraction {
            pub use crate::contracts::tag_extraction::{
                apply_tag_filter_on_subgraph_type_defs, apply_tag_filter_on_subgraphs,
                apply_tag_filter_to_inaccessible_transform_on_subgraph_schema, build_schema_coordinate_tag_register,
                SchemaCoordinateToTagsRegistry, TagFilterTransformResult, TransformTagDirectives,
            };
        }
    }

    /// `src/graphql/`.
    pub mod graphql {
        /// `src/graphql/printer.ts`.
        pub mod printer {
            pub use crate::graphql::printer::print;
        }

        /// `src/graphql/sort-sdl.ts`.
        pub mod sort_sdl {
            pub use crate::graphql::sort_sdl::sort_sdl;
        }

        /// `src/graphql/transform-supergraph-to-public-schema.ts`.
        pub mod transform_supergraph_to_public_schema {
            pub use crate::graphql::transform_supergraph_to_public_schema::transform_supergraph_to_public_schema;
        }
    }

    /// `src/specifications/`.
    pub mod specifications {
        /// `src/specifications/federation.ts`.
        pub mod federation {
            pub use crate::specifications::federation::FederationVersion;
        }

        /// `src/specifications/join.ts`.
        pub mod join {
            pub use crate::specifications::join::sdl;
        }

        /// `src/specifications/link.ts`.
        pub mod link {
            pub use crate::specifications::link::{
                parse_link_import, parse_link_url, print_link, Link, LinkImport, LinkUrl, SDL,
            };
        }

        /// `src/specifications/tag.ts`.
        pub mod tag {
            pub use crate::specifications::tag::DIRECTIVE;
        }
    }

    /// `src/subgraph/`.
    pub mod subgraph {
        /// `src/subgraph/context.ts`.
        pub mod context {
            pub use crate::subgraph::context::{parse_context_reference, ContextReference};
        }

        /// `src/subgraph/state.ts`.
        pub mod state {
            pub use crate::subgraph::state::{ArgumentKind, Deprecated, Description};
        }

        /// `src/subgraph/validation/`.
        pub mod validation {
            /// `src/subgraph/validation/validate-subgraph.ts`.
            pub mod validate_subgraph {
                pub use crate::subgraph::validation::validate_subgraph::InternalOptions;
            }
        }
    }

    /// `src/supergraph/`.
    pub mod supergraph {
        /// `src/supergraph/composition/`.
        pub mod composition {
            /// `src/supergraph/composition/ast.ts`.
            pub mod ast {
                pub use crate::supergraph::composition::ast::*;
            }
        }

        /// `src/supergraph/state.ts`.
        pub mod state {
            pub use crate::supergraph::state::SupergraphSpecs;
        }
    }

    /// `src/utils/`.
    pub mod utils {
        /// The collection types the port uses for JavaScript's `Map` and `Set`.
        pub mod collections {
            pub use crate::utils::collections::{IndexMap, IndexSet};
        }

        /// JavaScript semantics: thrown errors, `localeCompare`, number formatting.
        pub mod js {
            pub use crate::utils::js::{locale_compare, number_to_string, parse_float, parse_int, JsError, JsResult};
        }
    }

    /// `src/validate.ts`.
    pub mod validate {
        pub use crate::validate::{validate, validate_subgraph};
    }
}
