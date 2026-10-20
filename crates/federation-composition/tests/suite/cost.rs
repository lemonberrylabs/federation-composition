//! Port of `__tests__/cost.spec.ts`.
//!
//! The file installs a snapshot serializer for `specs.apollo.dev` strings, but has no snapshots.

use crate::shared::*;

test_versions! {
    fn cost_and_list_size_is_not_available_in_this_version(api, version) if "< v2.9" {}
}

// TS: the spec file's top-level `describe` block has the file's name.
#[allow(clippy::module_inception)]
mod cost {
    use super::*;

    test_versions! {
        fn cost_in_the_supergraph(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            enum UserType @cost(weight: 15) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @cost(weight: 20)
            }

            type User @cost(weight: 25) @key(fields: "id") {
              id: ID!
            }

            extend type User {
              name: String
            }

            type Query {
              userCount: Int @cost(weight: 5) @shareable
              user(id: ID! @cost(weight: 10)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version))]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/v0.5", for: EXECUTION)
          @link(url: "https://specs.apollo.dev/cost/v0.1") {
          query: Query
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        directive @cost(
          weight: Int!
        ) on ARGUMENT_DEFINITION | ENUM | FIELD_DEFINITION | INPUT_FIELD_DEFINITION | OBJECT | SCALAR
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @join__type(graph: FOO) @cost(weight: 15) {
          ADMIN @join__enumValue(graph: FOO)
          REGULAR @join__enumValue(graph: FOO)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        input UsersFilterInput @join__type(graph: FOO) {
          limit: Int @cost(weight: 20)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type User @join__type(graph: FOO, key: "id") @cost(weight: 25) {
          id: ID!
          name: String
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) {
          userCount: Int @cost(weight: 5)
          user(id: ID! @cost(weight: 10)): User
          userType(id: ID!): UserType
          users(filter: UsersFilterInput): [User]
        }
      "#);
        }

        fn cost_0(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@cost", "@key", "@shareable"]
              )

            enum UserType @cost(weight: 0) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @cost(weight: 0)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @cost(weight: 0) @shareable
              user(id: ID! @cost(weight: 0)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version))]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @join__type(graph: FOO) @cost(weight: 0) {
          ADMIN @join__enumValue(graph: FOO)
          REGULAR @join__enumValue(graph: FOO)
        }
      "#);

            to_contain_graphql(result.public_sdl(), r#"
        enum UserType {
          ADMIN
          REGULAR
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        input UsersFilterInput @join__type(graph: FOO) {
          limit: Int @cost(weight: 0)
        }
      "#);

            to_contain_graphql(result.public_sdl(), r#"
        input UsersFilterInput {
          limit: Int
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) {
          userCount: Int @cost(weight: 0)
          user(id: ID! @cost(weight: 0)): User
          userType(id: ID!): UserType
          users(filter: UsersFilterInput): [User]
        }
      "#);

            to_contain_graphql(result.public_sdl(), r#"
        type Query {
          userCount: Int
          user(id: ID!): User
          userType(id: ID!): UserType
          users(filter: UsersFilterInput): [User]
        }
      "#);
        }

        fn cost_0_2(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@cost", "@key", "@shareable"]
              )

            enum UserType @cost(weight: -1) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @cost(weight: -1)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @cost(weight: -1) @shareable
              user(id: ID! @cost(weight: -1)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version))]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType @join__type(graph: FOO) @cost(weight: -1) {
          ADMIN @join__enumValue(graph: FOO)
          REGULAR @join__enumValue(graph: FOO)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        input UsersFilterInput @join__type(graph: FOO) {
          limit: Int @cost(weight: -1)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) {
          userCount: Int @cost(weight: -1)
          user(id: ID! @cost(weight: -1)): User
          userType(id: ID!): UserType
          users(filter: UsersFilterInput): [User]
        }
      "#);
        }

        fn cost_on_shareables_be_math_max(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            enum UserType @cost(weight: 15) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @cost(weight: 20)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @cost(weight: 5) @shareable
              user(id: ID! @cost(weight: 10)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            enum UserType @cost(weight: 30) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @cost(weight: 40)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @cost(weight: 10) @shareable
              user(id: ID! @cost(weight: 20)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType
          @join__type(graph: FOO)
          @join__type(graph: BAR)
          @cost(weight: 30) {
          ADMIN @join__enumValue(graph: FOO) @join__enumValue(graph: BAR)
          REGULAR @join__enumValue(graph: FOO) @join__enumValue(graph: BAR)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        input UsersFilterInput @join__type(graph: FOO) @join__type(graph: BAR) {
          limit: Int @cost(weight: 40)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) @join__type(graph: BAR) {
          userCount: Int @cost(weight: 10)
          user(id: ID! @cost(weight: 20)): User
          userType(id: ID!): UserType
          users(filter: UsersFilterInput): [User]
        }
      "#);
        }

        fn incosistent_name_for_cost(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            enum UserType @cost(weight: 15) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @cost(weight: 20)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @cost(weight: 5) @shareable
              user(id: ID! @cost(weight: 15)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", { name: "@cost", as: "@price" }, "@shareable"]
              )

            enum UserType @price(weight: 30) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @price(weight: 40)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @price(weight: 10) @shareable
              user(id: ID! @price(weight: 20)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"The import name "@cost" is imported with mismatched name between subgraphs: it is imported as  "@price" in subgraph "bar" and  "@cost" in subgraph "foo""#)
                    .code("LINK_IMPORT_NAME_MISMATCH")],
            );
        }

        fn aliased_cost(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", { name: "@cost", as: "@price" }, "@shareable"]
              )

            enum UserType @price(weight: 15) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @price(weight: 20)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @price(weight: 5) @shareable
              user(id: ID! @price(weight: 10)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", { name: "@cost", as: "@price" }, "@shareable"]
              )

            enum UserType @price(weight: 30) {
              ADMIN
              REGULAR
            }

            input UsersFilterInput {
              limit: Int @price(weight: 40)
            }

            type User @key(fields: "id") {
              id: ID!
            }

            type Query {
              userCount: Int @price(weight: 10) @shareable
              user(id: ID! @price(weight: 20)): User @shareable
              userType(id: ID!): UserType @shareable
              users(filter: UsersFilterInput): [User] @shareable
            }
          "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/v0.5", for: EXECUTION)
          @link(
            url: "https://specs.apollo.dev/cost/v0.1"
            import: [{ name: "@cost", as: "@price" }]
          ) {
          query: Query
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        directive @price(
          weight: Int!
        ) on ARGUMENT_DEFINITION | ENUM | FIELD_DEFINITION | INPUT_FIELD_DEFINITION | OBJECT | SCALAR
      "#);

            assert_not_to_match_string(result.public_sdl(), "@price");
            assert_not_to_match_string(result.public_sdl(), "@cost");

            to_contain_graphql(result.supergraph_sdl(), r#"
        enum UserType
          @join__type(graph: FOO)
          @join__type(graph: BAR)
          @price(weight: 30) {
          ADMIN @join__enumValue(graph: FOO) @join__enumValue(graph: BAR)
          REGULAR @join__enumValue(graph: FOO) @join__enumValue(graph: BAR)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        input UsersFilterInput @join__type(graph: FOO) @join__type(graph: BAR) {
          limit: Int @price(weight: 40)
        }
      "#);

            to_contain_graphql(result.public_sdl(), r#"
          input UsersFilterInput {
            limit: Int
          }
        "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) @join__type(graph: BAR) {
          userCount: Int @price(weight: 10)
          user(id: ID! @price(weight: 20)): User
          userType(id: ID!): UserType
          users(filter: UsersFilterInput): [User]
        }
      "#);
        }

        fn cost_on_interface_type_is_not_allowed(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node @cost(weight: 5) {
              id: ID!
            }

            type User implements Node @cost(weight: 25) @key(fields: "id") {
              id: ID!
            }

            type Query {
              node(id: ID): Node
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] Directive "@cost" may not be used on INTERFACE."#)
                    .code("INVALID_GRAPHQL")],
            );
        }

        fn cost_on_interface_object_is_not_allowed(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node @key(fields: "id") {
              id: ID!
            }

            type User implements Node @cost(weight: 25) @key(fields: "id") {
              id: ID!
            }

            type Query {
              node(id: ID): Node
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@cost"])

            interface Node @key(fields: "id") @cost(weight: 5) {
              id: ID!
              name: String
            }
          "#, version)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[bar] Directive "@cost" may not be used on INTERFACE."#)
                    .code("INVALID_GRAPHQL")],
            );
        }

        fn cost_on_interface_field_is_not_allowed(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node {
              id: ID! @cost(weight: 5)
              name(substring: Int @cost(weight: 10)): String
            }

            type User implements Node @cost(weight: 25) @key(fields: "id") {
              id: ID!
              name(substring: Int @cost(weight: 15)): String
            }

            type Query {
              node(id: ID): Node
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] @cost cannot be applied to interface "Node.id""#)
                    .code("COST_APPLIED_TO_INTERFACE_FIELD")],
            );
        }

        fn cost_on_interface_field_argument_is_not_allowed(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node {
              id: ID!
              name(substring: Int @cost(weight: 10)): String
            }

            type User implements Node @cost(weight: 25) @key(fields: "id") {
              id: ID!
              name(substring: Int @cost(weight: 15)): String
            }

            type Query {
              node(id: ID): Node
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] @cost cannot be applied to interface "Node.name" argument "substring""#)
                    .code("COST_APPLIED_TO_INTERFACE_FIELD")],
            );
        }

        fn cost_on_interface_type_is_not_allowed_2(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node @cost(weight: 5) {
              id: ID!
            }

            type User implements Node @cost(weight: 25) @key(fields: "id") {
              id: ID!
            }

            type Query {
              node(id: ID): Node
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] Directive "@cost" may not be used on INTERFACE."#)
                    .code("INVALID_GRAPHQL")],
            );
        }

        fn cost_on_interface_object_field(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node @key(fields: "id") {
              id: ID! @cost(weight: 5)
            }

            type User implements Node @cost(weight: 25) @key(fields: "id") {
              id: ID!
            }

            type Query {
              node(id: ID): Node
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@interfaceObject", "@cost"]
              )

            type Node @key(fields: "id") @interfaceObject {
              id: ID! @cost(weight: 10)
              name(substring: Int @cost(weight: 10)): String
            }
          "#, version)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] @cost cannot be applied to interface "Node.id""#)
                    .code("COST_APPLIED_TO_INTERFACE_FIELD")],
            );
        }

        fn cost_on_object_type_implementing_interface(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node @key(fields: "id") {
              id: ID!
            }

            type User implements Node @key(fields: "id") @cost(weight: 5) {
              id: ID!
            }

            type Bot implements Node @key(fields: "id") @cost(weight: 10) {
              id: ID!
            }

            type Query {
              node(id: ID): Node
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost"]
              )

              type User @key(fields: "id") @cost(weight: 7) {
                id: ID!
              }
          "#, version)),
            ]);

            assert_composition_success(&result);

            // Should use highest cost among field implementations
            to_contain_graphql(result.supergraph_sdl(), r#"
        type User implements Node
          @join__implements(graph: FOO, interface: "Node")
          @join__type(graph: BAR, key: "id")
          @join__type(graph: FOO, key: "id")
          @cost(weight: 7) {
          id: ID!
        }
      "#);

            // shouldn't propagate to interface
            to_contain_graphql(result.supergraph_sdl(), r#"
        interface Node @join__type(graph: FOO, key: "id") {
          id: ID!
        }
      "#);
        }

        fn cost_on_object_type_fields_implementing_interface(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node @key(fields: "id") {
              id: ID!
            }

            type User implements Node @key(fields: "id") {
              id: ID! @cost(weight: 5)
            }

            type Bot implements Node @key(fields: "id") {
              id: ID! @cost(weight: 10)
            }

            type Query {
              node(id: ID): Node
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost"]
              )

              type User @key(fields: "id") {
                id: ID! @cost(weight: 7)
              }
          "#, version)),
            ]);

            assert_composition_success(&result);

            // Should use highest cost among field implementations
            to_contain_graphql(result.supergraph_sdl(), r#"
        type User implements Node
          @join__implements(graph: FOO, interface: "Node")
          @join__type(graph: BAR, key: "id")
          @join__type(graph: FOO, key: "id") {
          id: ID! @cost(weight: 7)
        }
      "#);

            // shouldn't propagate to interface
            to_contain_graphql(result.supergraph_sdl(), r#"
        interface Node @join__type(graph: FOO, key: "id") {
          id: ID!
        }
      "#);
        }

        fn cost_on_object_type_field_arguments_implementing_interface(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

            interface Node @key(fields: "id") {
              id: ID!
              name(substring: Int): String
            }

            type User implements Node @key(fields: "id") {
              id: ID!
              name(substring: Int @cost(weight: 5)): String @shareable
            }

            type Bot implements Node @key(fields: "id") {
              id: ID!
              name(substring: Int @cost(weight: 10)): String
            }

            type Query {
              node(id: ID): Node
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@cost", "@shareable"]
              )

              type User @key(fields: "id") {
                id: ID!
                name(substring: Int @cost(weight: 7)): String @shareable
              }
          "#, version)),
            ]);

            assert_composition_success(&result);

            // Should use highest cost among field implementations
            to_contain_graphql(result.supergraph_sdl(), r#"
        type User implements Node
          @join__implements(graph: FOO, interface: "Node")
          @join__type(graph: BAR, key: "id")
          @join__type(graph: FOO, key: "id") {
          id: ID!
          name(substring: Int @cost(weight: 7)): String
        }
      "#);

            // shouldn't propagate to interface
            to_contain_graphql(result.supergraph_sdl(), r#"
        interface Node @join__type(graph: FOO, key: "id") {
          id: ID!
          name(substring: Int): String
        }
      "#);
        }
    }
}

mod list_size {
    use super::*;

    test_versions! {
        fn list_size_on_interface_field(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@listSize", "@shareable"]
              )

            interface Node {
              id: ID!
              nodes(limit: Int!): [Node] @listSize(slicingArguments: ["limit"], sizedFields: ["nodes"], assumedSize: 21)
            }

            type User implements Node @key(fields: "id") {
              id: ID!
              nodes(limit: Int!): [Node] @listSize(slicingArguments: ["limit"], sizedFields: ["nodes"], assumedSize: 11) @shareable
            }

            type Query {
              node(id: ID): Node  @shareable
            }
          "#, version)),
                service("baz", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@listSize", "@shareable"]
              )

            interface Node {
              id: ID!
              nodes(limit: Int!): [Node] @listSize(slicingArguments: ["limit"], sizedFields: ["nodes"], assumedSize: 20)
            }

            type User implements Node @key(fields: "id") {
              id: ID!
              nodes(limit: Int!): [Node] @listSize(slicingArguments: ["limit"], sizedFields: ["nodes"], assumedSize: 10) @shareable
            }

            type Query {
              node(id: ID): Node @shareable
            }
          "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        interface Node @join__type(graph: BAR) @join__type(graph: BAZ) {
          id: ID!
          nodes(limit: Int!): [Node]
            @listSize(
              assumedSize: 21
              requireOneSlicingArgument: true
              sizedFields: ["nodes"]
              slicingArguments: ["limit"]
            )
        }
      "#);
        }

        fn in_supergraph(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type User {
              id: ID!
              name: String
            }

            type UserConnection {
              edges: [User]
              count: Int!
            }

            type Query {
              users(first: Int): UserConnection
                @listSize(slicingArguments: ["first"], sizedFields: ["edges"])
              allUsers: [User] @listSize(assumedSize: 1, requireOneSlicingArgument: false)
            }
          "#, version))]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        schema
          @link(for: EXECUTION, url: "https://specs.apollo.dev/join/v0.5")
          @link(
            url: "https://specs.apollo.dev/cost/v0.1"
            import: ["@listSize"]
          )
          @link(url: "https://specs.apollo.dev/link/v1.0") {
          query: Query
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        directive @listSize(
          assumedSize: Int
          slicingArguments: [String!]
          sizedFields: [String!]
          requireOneSlicingArgument: Boolean = true
        ) on FIELD_DEFINITION
      "#);

            assert_not_to_match_string(result.public_sdl(), "@listSize");

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) {
          allUsers: [User]
            @listSize(assumedSize: 1, requireOneSlicingArgument: false)
          users(first: Int): UserConnection
            @listSize(sizedFields: ["edges"], slicingArguments: ["first"])
        }
      "#);
        }

        fn aliased(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: [{ name: "@listSize", as: "@cartSize" }]
              )

            type User {
              id: ID!
              name: String
            }

            type UserConnection {
              edges: [User]
              count: Int!
            }

            type Query {
              users(first: Int): UserConnection
                @cartSize(slicingArguments: ["first"], sizedFields: ["edges"])
              allUsers: [User] @cartSize(assumedSize: 1, requireOneSlicingArgument: false)
            }
          "#, version))]);

            assert_composition_success(&result);

            assert_not_to_match_string(result.public_sdl(), "@listSize");
            assert_not_to_match_string(result.public_sdl(), "@cartSize");

            to_contain_graphql(result.supergraph_sdl(), r#"
        schema
          @link(for: EXECUTION, url: "https://specs.apollo.dev/join/v0.5")
          @link(
            url: "https://specs.apollo.dev/cost/v0.1"
            import: [{ name: "@listSize", as: "@cartSize" }]
          )
          @link(url: "https://specs.apollo.dev/link/v1.0") {
          query: Query
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        directive @cartSize(
          assumedSize: Int
          slicingArguments: [String!]
          sizedFields: [String!]
          requireOneSlicingArgument: Boolean = true
        ) on FIELD_DEFINITION
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) {
          allUsers: [User]
            @cartSize(assumedSize: 1, requireOneSlicingArgument: false)
          users(first: Int): UserConnection
            @cartSize(sizedFields: ["edges"], slicingArguments: ["first"])
        }
      "#);
        }

        fn inconsistent_name(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: [{ name: "@listSize", as: "@cartSize" }]
              )

            type User {
              id: ID!
              name: String
            }

            type UserConnection {
              edges: [User]
              count: Int!
            }

            type Query {
              users(first: Int): UserConnection
                @cartSize(slicingArguments: ["first"], sizedFields: ["edges"])
              allUsers: [User] @cartSize(assumedSize: 1, requireOneSlicingArgument: false)
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type Query {
              words: [String] @listSize(assumedSize: 5)
            }
          "#, version)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"The import name "@listSize" is imported with mismatched name between subgraphs: it is imported as  "@listSize" in subgraph "bar" and  "@cartSize" in subgraph "foo""#)
                    .code("LINK_IMPORT_NAME_MISMATCH")],
            );
        }

        fn mixing_assumed_size_and_sized_fields(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: Int!
            }

            type Query {
              strings(first: Int): StringConnection
                @listSize(slicingArguments: ["first"], sizedFields: ["edges"], assumedSize: 1000)
            }
          "#, version))]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) {
          strings(first: Int): StringConnection
            @listSize(
              slicingArguments: ["first"]
              sizedFields: ["edges"]
              assumedSize: 1000
            )
        }
      "#);
        }

        fn non_shared_slicing_argument(api, version) if ">= v2.9" {
            let services = [
                service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize", "@shareable"])

            type StringConnection {
              edges: [String] @shareable
              count: Int! @shareable
            }

            type Query {
              strings(first: Int): StringConnection @shareable
                @listSize(slicingArguments: ["first"], sizedFields: ["edges"], assumedSize: 1000, requireOneSlicingArgument: true)
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize", "@shareable"])

            type StringConnection {
              edges: [String] @shareable
              count: Int! @shareable
            }

            type Query {
              strings(first: Int, limit: Int): StringConnection @shareable
                @listSize(slicingArguments: ["limit"], sizedFields: ["edges"], assumedSize: 2000, requireOneSlicingArgument: false)
            }
          "#, version)),
            ];

            // Slicing argument "limit" is not available in all subgraphs

            // Our composition allows it and strips out the non-shared slicing argument
            let result = api.compose_services(&services);
            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: BAR) @join__type(graph: FOO) {
          strings(first: Int): StringConnection
            @listSize(
              slicingArguments: ["first"]
              sizedFields: ["edges"]
              assumedSize: 2000
              requireOneSlicingArgument: false
            )
        }
      "#);
        }

        fn interface_field(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            interface Node {
              id: ID!
            }

            interface NodeConnection {
              edges: [Node] @listSize(assumedSize: 2)
              count: Int!
            }

            type User implements Node {
              id: ID!
              name: String
            }

            type UserConnection implements NodeConnection {
              edges: [User] @listSize(assumedSize: 1)
              count: Int!
            }

            type Query {
              nodes(first: Int): NodeConnection
                @listSize(slicingArguments: ["first"], sizedFields: ["edges"])
              allNodes: [Node] @listSize(assumedSize: 1, requireOneSlicingArgument: false)
            }
          "#, version))]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        interface NodeConnection @join__type(graph: FOO) {
          count: Int!
          edges: [Node] @listSize(assumedSize: 2)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type UserConnection implements NodeConnection
          @join__implements(graph: FOO, interface: "NodeConnection")
          @join__type(graph: FOO) {
          edges: [User] @listSize(assumedSize: 1)
          count: Int!
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Query @join__type(graph: FOO) {
          nodes(first: Int): NodeConnection
            @listSize(slicingArguments: ["first"], sizedFields: ["edges"])
          allNodes: [Node]
            @listSize(assumedSize: 1, requireOneSlicingArgument: false)
        }
      "#);
        }

        fn interface_object_field(api, version) if ">= v2.9" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@listSize", "@shareable"]
              )

            interface Node @key(fields: "id") {
              id: ID!
            }

            type User implements Node @key(fields: "id") {
              id: ID!
            }

            type Query {
              node(id: ID): Node
            }
          "#, version)),
                service("bar", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@interfaceObject", "@listSize"]
              )

            type Node @key(fields: "id") @interfaceObject {
              id: ID!
              accessList: [String] @listSize(assumedSize: 10)
            }
          "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        interface Node
          @join__type(graph: BAR, key: "id", isInterfaceObject: true)
          @join__type(graph: FOO, key: "id") {
          id: ID!
          accessList: [String]
            @join__field(graph: BAR)
            @listSize(assumedSize: 10)
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type User implements Node
          @join__implements(graph: FOO, interface: "Node")
          @join__type(graph: FOO, key: "id") {
          id: ID!
          accessList: [String] @listSize(assumedSize: 10) @join__field
        }
      "#);
        }

        fn negative_assumed_size(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: Int!
            }

            type Query {
              strings(first: Int): StringConnection
                @listSize(assumedSize: -1, sizedFields: ["edges"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" has negative @listSize(assumedSize:) value"#)
                    .code("LIST_SIZE_INVALID_ASSUMED_SIZE")],
            );
        }

        fn zero_assumed_size(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: Int!
            }

            type Query {
              strings(first: Int): StringConnection
                @listSize(assumedSize: 0, sizedFields: ["edges"])
            }
          "#, version))]);

            assert_composition_success(&result);
        }

        fn non_list_field_with_the_sized_fields_argument(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: Int!
            }

            type Query {
              strings(first: Int): StringConnection
                @listSize(slicingArguments: ["first"], sizedFields: ["count"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" references "count" field in @listSize(sizedFields:) argument that is not a list."#)
                    .code("LIST_SIZE_APPLIED_TO_NON_LIST")],
            );
        }

        fn non_list_field_without_the_sized_fields_argument(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: Int!
            }

            type Query {
              strings(first: String): StringConnection @listSize(slicingArguments: ["first"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" is not a list. Try to add @listSize(sizedFields:) argument."#)
                    .code("LIST_SIZE_INVALID_SIZED_FIELD")],
            );
        }

        fn sized_fields_on_non_object_output(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type Query {
              strings(first: Int): [String]
                @listSize(slicingArguments: ["first"], sizedFields: ["edges"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" has @listSize(sizedFields:) applied, but the output type is not a composite type"#)
                    .code("LIST_SIZE_INVALID_SIZED_FIELD")],
            );
        }

        fn sized_field_should_be_a_list(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: String!
            }

            type Query {
              strings(first: Int): StringConnection
                @listSize(slicingArguments: ["first"], sizedFields: ["count"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" references "count" field in @listSize(sizedFields:) argument that is not a list."#)
                    .code("LIST_SIZE_APPLIED_TO_NON_LIST")],
            );
        }

        fn sized_field_should_reference_existing_field(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: String!
            }

            type Query {
              strings(first: Int): StringConnection
                @listSize(slicingArguments: ["first"], sizedFields: ["discount"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" references "discount" field in @listSize(sizedFields:) argument that does not exist."#)
                    .code("LIST_SIZE_INVALID_SIZED_FIELD")],
            );
        }

        fn slicing_argument_should_be_int_or_int(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: Int!
            }

            type Query {
              strings(first: String): StringConnection
                @listSize(slicingArguments: ["first"], sizedFields: ["edges"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" references "first" argument in @listSize(slicingArguments:) that is not an integer."#)
                    .code("LIST_SIZE_INVALID_SLICING_ARGUMENT")],
            );
        }

        fn slicing_argument_should_reference_existing_field(api, version) if ">= v2.9" {
            let result = api.compose_services(&[service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@listSize"])

            type StringConnection {
              edges: [String]
              count: Int!
            }

            type Query {
              strings(first: Int): StringConnection
                @listSize(slicingArguments: ["last"], sizedFields: ["edges"])
            }
          "#, version))]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] "Query.strings" references "last" argument in @listSize(slicingArguments:) that does not exist."#)
                    .code("LIST_SIZE_INVALID_SLICING_ARGUMENT")],
            );
        }

        #[ignore = "todo in TypeScript"]
        fn require_one_slicing_argument_when_slicing_arguments_is_empty(api, version) if ">= v2.9" {}

        #[ignore = "todo in TypeScript"]
        fn require_one_slicing_argument_when_slicing_arguments_is_not_defined(api, version) if ">= v2.9" {}
    }
}
