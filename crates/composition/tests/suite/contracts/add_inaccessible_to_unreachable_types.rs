//! Port of `__tests__/contracts/add-inaccessible-to-unreachable-types.spec.ts`.

use crate::shared::*;

use federation_composition::{add_inaccessible_to_unreachable_types, extract_link_implementations};

/// `compositionResult = addInaccessibleToUnreachableTypes(resolveImportName, compositionResult as CompositionSuccess)`,
/// with `resolveImportName` from `extractLinkImplementations(parse(compositionResult.supergraphSdl!))`.
fn add_inaccessible(composition_result: &CompositionResult) -> CompositionSuccess {
    let CompositionResult::Success(success) = composition_result else {
        panic!("expected a CompositionSuccess");
    };
    let link_implementations = extract_link_implementations(&parse(success.supergraph_sdl()).unwrap()).unwrap();
    let resolve_import_name =
        |identity: &str, name: &str| link_implementations.resolve_import_name(identity, name);
    add_inaccessible_to_unreachable_types(&resolve_import_name, success).unwrap()
}

#[test]
fn filters_based_on_tags() {
    let composition_result = compose_services(
        &[service_with_url(
            "user",
            "https://user-example.graphql-hive.com",
            parse(
                r#"
        type Query {
          user: User
        }
        type User {
          id: ID!
          name: String
        }

        type Unused {
          foo: String
        }
      "#,
            )
            .unwrap(),
        )],
        None,
    );

    assert_eq!(composition_result.errors(), None);
    assert!(composition_result.supergraph_sdl().is_some());

    let composition_result = add_inaccessible(&composition_result);

    to_match_inline_snapshot(
        composition_result.supergraph_sdl(),
        r#"
    "schema @link(url: "https://specs.apollo.dev/link/v1.0") @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION) {
      query: Query
    }

    directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

    directive @join__graph(name: String!, url: String!) on ENUM_VALUE

    directive @join__field(
      graph: join__Graph
      requires: join__FieldSet
      provides: join__FieldSet
      type: String
      external: Boolean
      override: String
      usedOverridden: Boolean
    ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

    directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE

    directive @join__type(
      graph: join__Graph!
      key: join__FieldSet
      extension: Boolean! = false
      resolvable: Boolean! = true
      isInterfaceObject: Boolean! = false
    ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

    directive @join__unionMember(graph: join__Graph!, member: String!) repeatable on UNION

    scalar join__FieldSet

    directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA

    scalar link__Import

    enum link__Purpose {
      """
      `SECURITY` features provide metadata necessary to securely resolve fields.
      """
      SECURITY
      """
      `EXECUTION` features provide metadata necessary for operation execution.
      """
      EXECUTION
    }

    enum join__Graph {
      USER @join__graph(name: "user", url: "https://user-example.graphql-hive.com")
    }

    type Query @join__type(graph: USER) {
      user: User
    }

    type User @join__type(graph: USER) {
      id: ID!
      name: String
    }

    type Unused @join__type(graph: USER) @inaccessible {
      foo: String
    }"
  "#,
    );
    to_match_inline_snapshot(
        composition_result.public_sdl().unwrap(),
        r#"
    "type Query {
      user: User
    }

    type User {
      id: ID!
      name: String
    }"
  "#,
    );
}

#[test]
fn inaccessible_is_not_added_on_built_in_federation_and_context_types() {
    let sdl = r#"
    extend schema
      @link(
        url: "https://specs.apollo.dev/federation/v2.9"
        import: ["@context", "@fromContext", "@key"]
      )

    type Query {
      me: Me
    }

    type Me @key(fields: "id") @context(name: "me") {
      id: ID!
      wallet: Wallet
    }

    type Wallet @key(fields: "id") {
      id: ID!
      contextualField(value: ID @fromContext(field: "$me { id }")): String
    }
  "#;

    let composition_result = compose_services(
        &[service_with_url("user", "https://noop.graphql-hive.com", parse(sdl).unwrap())],
        None,
    );

    assert_eq!(composition_result.errors(), None);
    assert!(composition_result.supergraph_sdl().is_some());

    let composition_result = add_inaccessible(&composition_result);
    let supergraph_sdl = composition_result.supergraph_sdl();

    assert!(supergraph_sdl.contains("scalar join__FieldValue\n"));
    assert!(!supergraph_sdl.contains("scalar join__FieldValue @inaccessible"));
    assert!(!supergraph_sdl.contains("input join__ContextArgument @inaccessible {"));
    assert!(supergraph_sdl.contains("input join__ContextArgument {\n"));
    assert!(supergraph_sdl.contains("scalar context__ContextFieldValue\n"));
    assert!(!supergraph_sdl.contains("scalar context__ContextFieldValue @inaccessible"));
}
