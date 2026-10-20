# federation-composition

A Rust port of [`@theguild/federation-composition`](https://github.com/graphql-hive/federation-composition), the Apollo Federation v1/v2 schema composition library by [The Guild](https://the-guild.dev). Given a set of subgraph schemas, it validates them and composes the supergraph schema, with the same output and error messages as the TypeScript library: the supergraph SDL is byte for byte what the TypeScript produces, and errors have the same message, `extensions.code` and order.

GraphQL documents are parsed and printed with the `graphql-core` crate, a port of graphql-js from the same repository, which this crate re-exports as `federation_composition::graphql_core`.

## Usage

```toml
[dependencies]
federation-composition = "0.1"
```

Compose two subgraphs into a supergraph:

```rust
use federation_composition::{compose_services, graphql_core, CompositionResult, ServiceDefinition};

let users = ServiceDefinition::new(
    "users",
    graphql_core::parse(
        r#"
        extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])

        type Query {
          me: User
        }

        type User @key(fields: "id") {
          id: ID!
          name: String
        }
        "#,
    )?,
)
.with_url("http://users.internal/graphql");

let reviews = ServiceDefinition::new(
    "reviews",
    graphql_core::parse(
        r#"
        extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@key"])

        type User @key(fields: "id") {
          id: ID!
          reviews: [Review!]!
        }

        type Review {
          body: String!
        }
        "#,
    )?,
);

match compose_services(&[users, reviews])? {
    CompositionResult::Success(success) => {
        // The supergraph, for a router.
        let supergraph_sdl = success.supergraph_sdl();
        assert!(supergraph_sdl.contains(r#"USERS @join__graph(name: "users", url: "http://users.internal/graphql")"#));
        // The API schema clients see, without the federation machinery.
        let public_sdl = success.public_sdl()?;
        assert!(public_sdl.contains("reviews: [Review!]!"));
        assert!(!public_sdl.contains("join__"));
    }
    CompositionResult::Failure(failure) => panic!("composition failed:\n{failure}"),
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

When the subgraphs do not compose, the result is a `CompositionResult::Failure` whose errors carry the TypeScript library's message and code:

```rust
use federation_composition::{compose_services, graphql_core, ServiceDefinition};

let subgraph = |name: &str, sdl: &str| -> Result<ServiceDefinition, graphql_core::GraphQLError> {
    Ok(ServiceDefinition::new(name, graphql_core::parse(sdl)?))
};
let result = compose_services(&[
    subgraph("a", "type Query { price: Int }")?,
    subgraph("b", "type Query { price: String }")?,
])?;

let errors = result.errors().expect("the field types differ");
assert!(errors.iter().any(|error| error.code() == Some("FIELD_TYPE_MISMATCH")));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`CompositionResult::into_result` turns the result into a `Result<CompositionSuccess, CompositionFailure>`; `CompositionFailure` implements `std::error::Error`, so `?` works on it.

### Errors

There are two kinds of errors, as in the TypeScript library:

- **Composition errors** are the expected outcome of composing incompatible subgraphs. They are returned in a `CompositionFailure` (or a failed `ValidationResult`) as `GraphQLError`s, each with an `extensions.code` such as `FIELD_TYPE_MISMATCH` or `SATISFIABILITY_ERROR`.
- **`Error`** is returned where the TypeScript library throws instead, for example for two subgraphs with the same name or an invalid `@link` URL, and for a document nested deeper than `graphql_core::MAX_NESTING_DEPTH` levels, which only a document built in code can be ([SECURITY.md](https://github.com/lemonberrylabs/federation-composition/blob/main/SECURITY.md#untrusted-input)). It implements `std::error::Error`, `Send` and `Sync`; `Error::code` and `Error::graphql_error` give the code and the GraphQL error when the thrown value is one.

### Schema contracts

```rust
use federation_composition::{compose_schema_contract, graphql_core, SchemaContractFilter, ServiceDefinition};

let products = ServiceDefinition::new(
    "products",
    graphql_core::parse(
        r#"
        extend schema @link(url: "https://specs.apollo.dev/federation/v2.3", import: ["@tag"])

        type Query {
          products: [Product!]! @tag(name: "public")
          internalStats: Int
        }

        type Product {
          name: String! @tag(name: "public")
          cost: Int
        }
        "#,
    )?,
);

let filter = SchemaContractFilter::new(["public"], Vec::<String>::new());
let contract = compose_schema_contract(&[products], &filter, true)?
    .into_result()?;
let public_sdl = contract.public_sdl()?;
assert!(public_sdl.contains("products: [Product!]!"));
assert!(!public_sdl.contains("internalStats"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API

The public API is the TypeScript library's `src/index.ts` export list, with Rust naming:

| TypeScript | Rust |
|---|---|
| `composeServices(services)` | `compose_services` → `CompositionResult` |
| `CompositionResult`, `CompositionSuccess`, `CompositionFailure` | `CompositionResult`, `CompositionSuccess`, `CompositionFailure` |
| `assertCompositionSuccess`, `assertCompositionFailure`, `compositionHasErrors` | `assert_composition_success`, `assert_composition_failure`, `composition_has_errors` |
| `createCompositionSuccessReadCacheContainer` | `create_composition_success_read_cache_container` |
| `ServiceDefinition` | `ServiceDefinition` |
| `validate(subgraphs)`, `validateSubgraph(subgraph)` | `validate` → `ValidationResult`, `validate_subgraph` |
| `extractLinkImplementations`, `LinkVersion`, `FEDERATION_V1` | `extract_link_implementations` → `LinkImplementations`, `LinkVersion`, `FEDERATION_V1` |
| `transformSupergraphToPublicSchema`, `containsSupergraphSpec`, `sortSDL` | `transform_supergraph_to_public_schema`, `contains_supergraph_spec`, `sort_sdl` |
| `composeSchemaContract`, `addInaccessibleToUnreachableTypes`, `applyTagFilterOnSubgraphs` | `compose_schema_contract`, `add_inaccessible_to_unreachable_types`, `apply_tag_filter_on_subgraphs` with `SchemaContractFilter` |

The TypeScript functions' `__internal` options (such as `disableValidationRules`) are not part of the Rust API. The crate also has a hidden `__internal` module, used by this repository's tests and WebAssembly bindings; it is not part of the public API and can change in any release.

## Credits

This crate is a port of `@theguild/federation-composition`, created and maintained by The Guild, and keeps its MIT license. Thanks to The Guild for the original library and for encouraging this port. See the `LICENSE` file.
