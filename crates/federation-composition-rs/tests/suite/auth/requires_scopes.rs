//! Port of `__tests__/auth/requires-scopes.spec.ts`.

use crate::shared::*;

mod requires_scopes {
    use super::*;

    test_implementations! {
        fn forbidden_on_interface_type(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            interface Node @requiresScopes(scopes: [["a"]]) {
              id: ID!
            }

            type User implements Node @key(fields: "id") {
              id: ID!
            }

            type Query {
              node(id: ID!): Node
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Invalid use of @requiresScopes on interface "Node": @requiresScopes cannot be applied on interfaces, interface fields and interface objects"#)
                .code("AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE")]);
        }

        fn forbidden_on_interface_field(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            interface Node {
              id: ID! @requiresScopes(scopes: [["a"]])
            }

            type User implements Node @key(fields: "id") {
              id: ID!
            }

            type Query {
              node(id: ID!): Node
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Invalid use of @requiresScopes on field "Node.id": @requiresScopes cannot be applied on interfaces, interface fields and interface objects"#)
                .code("AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE")]);
        }

        fn forbidden_on_interface_object(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key", "@interfaceObject"]
              )

            type User
              @key(fields: "id")
              @interfaceObject
              @requiresScopes(scopes: [["a"]]) {
              id: ID!
              name: String!
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            interface User @key(fields: "id") {
              id: ID!
            }

            type Authenticated implements User @key(fields: "id") {
              id: ID!
            }

            type Query {
              User(id: ID!): User
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Invalid use of @requiresScopes on interface object "User": @requiresScopes cannot be applied on interfaces, interface fields and interface objects"#)
                .code("AUTH_REQUIREMENTS_APPLIED_ON_INTERFACE")]);
        }

        fn allowed_on_interface_object_field(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: [
                  "@requiresScopes"
                  "@key"
                  "@interfaceObject"
                  "@shareable"
                ]
              )

            type User @key(fields: "id") @interfaceObject {
              id: ID!
              name: String @requiresScopes(scopes: [["a"]]) @shareable # REF_AUTH_INTERFACE_OBJECT_SHAREABLE
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key", "@shareable"]
              )

            interface User @key(fields: "id") {
              id: ID!
            }

            type Authenticated implements User @key(fields: "id") {
              id: ID!
            }

            type Temorary implements User @key(fields: "id") {
              id: ID!
              name: String @requiresScopes(scopes: [["c"]]) @shareable
            }

            type Query {
              user(id: ID!): User
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        interface User
          @join__type(graph: A, key: "id", isInterfaceObject: true)
          @join__type(graph: B, key: "id") {
          id: ID!
          name: String
            @join__field(graph: A)
            @requiresScopes(scopes: [["a", "c"]])
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type Authenticated implements User
          @join__implements(graph: B, interface: "User")
          @join__type(graph: B, key: "id") {
          id: ID!
          name: String @requiresScopes(scopes: [["a"]]) @join__field
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type Temorary implements User
          @join__implements(graph: B, interface: "User")
          @join__type(graph: B, key: "id") {
          id: ID!
          name: String @requiresScopes(scopes: [["a", "c"]])
        }
      "#));
        }

        fn allowed_on_interface_object_field_that_belongs_to_interface(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key", "@interfaceObject"]
              )

            type User @key(fields: "id") @interfaceObject {
              id: ID! @requiresScopes(scopes: [["a"]])
              name: String
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            interface User @key(fields: "id") {
              id: ID!
            }

            type Authenticated implements User @key(fields: "id") {
              id: ID!
            }

            type Query {
              user(id: ID!): User
            }
          "#)),
                service("c", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key", "@interfaceObject"]
              )

            type User @key(fields: "id") @interfaceObject {
              id: ID! @requiresScopes(scopes: [["c"]])
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type Authenticated implements User
          @join__implements(graph: B, interface: "User")
          @join__type(graph: B, key: "id") {
          id: ID! @requiresScopes(scopes: [["a", "c"]])
          name: String @join__field
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        interface User
          @join__type(graph: A, isInterfaceObject: true, key: "id")
          @join__type(graph: B, key: "id")
          @join__type(graph: C, isInterfaceObject: true, key: "id") {
          id: ID! @requiresScopes(scopes: [["a", "c"]])
          name: String @join__field(graph: A)
        }
      "#));
        }

        fn forbidden_on_input_object_type(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            input UserInput @requiresScopes(scopes: [["a"]]) {
              id: ID!
            }

            type Query {
              user(input: UserInput): String
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Directive "@requiresScopes" may not be used on INPUT_OBJECT."#)
                .code("INVALID_GRAPHQL")]);
        }

        fn forbidden_on_union_type(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            union Result @requiresScopes(scopes: [["a"]]) = User | Error

            type User @key(fields: "id") {
              id: ID!
            }

            type Error {
              message: String
            }

            type Query {
              result: Result
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Directive "@requiresScopes" may not be used on UNION."#)
                .code("INVALID_GRAPHQL")]);
        }

        fn allowed_on_scalar(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes"]
              )

            scalar JSON @requiresScopes(scopes: [["a"]])

            type Query {
              json: JSON
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        scalar JSON @join__type(graph: A) @requiresScopes(scopes: [["a"]])
      "#));
        }

        fn allowed_on_enum(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes"]
              )

            enum Role @requiresScopes(scopes: [["a"]]) {
              ADMIN
            }

            type Query {
              role: Role
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        enum Role @join__type(graph: A) @requiresScopes(scopes: [["a"]]) {
          ADMIN @join__enumValue(graph: A)
        }
      "#));
        }

        fn passed_from_object_types_implementing_the_interface_to_interface_type(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            interface Node {
              id: ID!
            }

            type User implements Node
              @key(fields: "id")
              @requiresScopes(scopes: [["a"]]) {
              id: ID!
            }

            type Post implements Node
              @key(fields: "id")
              @requiresScopes(scopes: [["b"]]) {
              id: ID!
            }

            type Query {
              node: Node
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        interface Node
          @join__type(graph: A)
          @requiresScopes(scopes: [["a", "b"]]) {
          id: ID!
        }
      "#));
        }

        fn passed_from_object_fields_implementing_the_interface_to_interface_field(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            interface Node {
              id: ID!
              name: String
            }

            type User implements Node
              @key(fields: "id")
              @requiresScopes(scopes: [["a"]]) {
              id: ID!
              name: String @requiresScopes(scopes: [["a"]])
            }

            type Post implements Node @key(fields: "id") {
              id: ID!
              name: String @requiresScopes(scopes: [["b"]])
            }

            type Query {
              node: Node
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        interface Node @join__type(graph: A) @requiresScopes(scopes: [["a"]]) {
          id: ID!
          name: String @requiresScopes(scopes: [["a", "b"]])
        }
      "#));
        }

        fn many_object_type_instances_of_the_same_name_across_subgraphs(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type User @key(fields: "id") @requiresScopes(scopes: [["a"]]) {
              id: ID!
            }

            type Query {
              user: User
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type User @key(fields: "id") @requiresScopes(scopes: [["b"]]) {
              id: ID!
              name: String
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id")
          @requiresScopes(scopes: [["a", "b"]]) {
          id: ID!
          name: String @join__field(graph: B)
        }
      "#));
        }

        fn many_object_fields_instances_of_the_same_name_across_subgraphs(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key", "@shareable"]
              )

            type User @key(fields: "id") {
              id: ID!
              name: String @requiresScopes(scopes: [["a"]]) @shareable
            }

            type Query {
              user: User
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key", "@shareable"]
              )

            type User @key(fields: "id") {
              id: ID!
              name: String @requiresScopes(scopes: [["b"]]) @shareable
              age: Int
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID!
          age: Int @join__field(graph: B)
          name: String @requiresScopes(scopes: [["a", "b"]])
        }
      "#));
        }

        fn deduplicated_and_dropped_redundant_scopes(api) {
            let result = api.compose_services(&[
                service("accounts", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type User
              @key(fields: "id")
              @requiresScopes(
                scopes: [["user:read", "user:email:read"], ["admin"]]
              ) {
              id: ID!
            }

            type Query {
              user: User
            }
          "#)),
                service("billing", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type User
              @key(fields: "id")
              @requiresScopes(
                scopes: [
                  ["user:read", "billing:read"]
                  ["admin", "billing:invoice:read"]
                  ["support:user:read"]
                ]
              ) {
              id: ID!
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type User
          @join__type(graph: ACCOUNTS, key: "id")
          @join__type(graph: BILLING, key: "id")
          @requiresScopes(
            scopes: [
              ["admin", "billing:invoice:read"]
              ["admin", "support:user:read"]
              ["admin", "billing:read", "user:read"]
              ["billing:read", "user:email:read", "user:read"]
              ["support:user:read", "user:email:read", "user:read"]
            ]
          ) {
          id: ID!
        }
      "#));
        }

        fn requires_missing_access_to_referenced_interface_field(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@key", "@external", "@requires"]
              )

            type Query {
              user: User
            }

            type User @key(fields: "id") {
              id: ID
              comment: Node @external
              hasComment: Boolean @requires(fields: "comment { id }")
            }

            interface Node {
              id: ID!
            }

            type Comment implements Node @external {
              id: ID!
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type User @key(fields: "id") {
              id: ID!
              comment: Node
            }

            interface Node {
              id: ID!
            }

            type Comment implements Node {
              id: ID! @requiresScopes(scopes: [["comment"]])
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Field "User.hasComment" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive field "Node.id" data from @requires selection set."#)
                .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")]);
        }

        fn requires_has_access_to_referenced_interface_field(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@key", "@external", "@requires", "@requiresScopes"]
              )

            type Query {
              user: User
            }

            type User @key(fields: "id") @requiresScopes(scopes: [["user"]]) {
              id: ID
              comment: Node @external
              hasComment: Boolean
                @requires(fields: "comment { id }")
                # combination of type and field access
                # that equals to the required access on the referenced field
                @requiresScopes(scopes: [["comment"]])
            }

            interface Node {
              id: ID!
            }

            type Comment implements Node @external {
              id: ID!
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type User @key(fields: "id") {
              id: ID!
              comment: Node
            }

            interface Node {
              id: ID!
            }

            type Comment implements Node {
              id: ID! @requiresScopes(scopes: [["comment", "user"]])
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type Comment implements Node
          @join__implements(graph: A, interface: "Node")
          @join__implements(graph: B, interface: "Node")
          @join__type(graph: A)
          @join__type(graph: B) {
          id: ID!
            @join__field(graph: A, external: true)
            @join__field(graph: B)
            @requiresScopes(scopes: [["comment", "user"]])
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        interface Node @join__type(graph: A) @join__type(graph: B) {
          id: ID! @requiresScopes(scopes: [["comment", "user"]])
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id")
          @requiresScopes(scopes: [["user"]]) {
          id: ID
            @join__field(graph: A, type: "ID")
            @join__field(graph: B, type: "ID!")
          comment: Node
            @join__field(graph: A, external: true)
            @join__field(graph: B)
          hasComment: Boolean
            @join__field(graph: A, requires: "comment { id }")
            @requiresScopes(scopes: [["comment"]])
        }
      "#));
        }

        fn requires_missing_access_to_referenced_interface_type(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@key", "@external", "@requires"]
              )

            type Query {
              account: Account
            }

            type Account @key(fields: "id") {
              id: ID
              details: AccountDetail @external
              detailsSummary: String
                @requires(
                  fields: "details { ... on ScopedAccountDetail { sensitiveLevel } ... on PublicAccountDetail { publicNote } ... on AdminAccountDetail { adminLevel } }"
                )
            }

            interface AccountDetail {
              summary: String
            }

            type ScopedAccountDetail implements AccountDetail @external {
              summary: String
              sensitiveLevel: Int
            }

            type PublicAccountDetail implements AccountDetail @external {
              summary: String
              publicNote: String
            }

            type AdminAccountDetail implements AccountDetail @external {
              summary: String
              adminLevel: Int
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type Account @key(fields: "id") {
              id: ID
              details: AccountDetail
            }

            interface AccountDetail {
              summary: String
            }

            type ScopedAccountDetail implements AccountDetail
              @requiresScopes(scopes: [["missing"]]) {
              summary: String
              sensitiveLevel: Int
            }

            type PublicAccountDetail implements AccountDetail {
              summary: String
              publicNote: String
            }

            type AdminAccountDetail implements AccountDetail
              @requiresScopes(scopes: [["missing"]]) {
              summary: String
              adminLevel: Int
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Field "Account.detailsSummary" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive field "Account.details" data from @requires selection set."#)
                .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")]);
        }

        fn requires_missing_access_to_referenced_interface_type_2(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@key", "@external", "@requires", "@requiresScopes"]
              )

            type Query {
              account: Account
            }

            type Account
              @key(fields: "id")
              @requiresScopes(scopes: [["account"]]) {
              id: ID
              details: AccountDetail @external
              detailsSummary: String
                @requires(
                  fields: "details { summary ... on ScopedAccountDetail { sensitiveLevel } ... on PublicAccountDetail { publicNote } ... on AdminAccountDetail { adminLevel } }"
                )
                @requiresScopes(scopes: [["scoped:summary", "scoped:level"]])
            }

            interface AccountDetail {
              summary: String
            }

            type ScopedAccountDetail implements AccountDetail @external {
              summary: String
              sensitiveLevel: Int
            }

            type PublicAccountDetail implements AccountDetail @external {
              summary: String
              publicNote: String
            }

            type AdminAccountDetail implements AccountDetail @external {
              summary: String
              adminLevel: Int
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type Account @key(fields: "id") {
              id: ID
              details: AccountDetail
                @requiresScopes(scopes: [["account:details"]])
            }

            interface AccountDetail {
              summary: String
            }

            type ScopedAccountDetail implements AccountDetail {
              summary: String @requiresScopes(scopes: [["scoped:summary"]])
              sensitiveLevel: Int @requiresScopes(scopes: [["scoped:level"]])
            }

            type PublicAccountDetail implements AccountDetail {
              summary: String
              publicNote: String @requiresScopes(scopes: [["public:note"]])
            }

            type AdminAccountDetail implements AccountDetail {
              summary: String @requiresScopes(scopes: [["admin:summary"]])
              adminLevel: Int @requiresScopes(scopes: [["admin:level"]])
            }
          "#)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(result.errors(), &[ErrorMatcher::new()
                .message(r#"[a] Field "Account.detailsSummary" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive field "Account.details" data from @requires selection set."#)
                .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")]);
        }

        fn requires_has_access_to_referenced_interface_type(api) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@key", "@external", "@requires", "@requiresScopes"]
              )

            type Query {
              account: Account
            }

            type Account
              @key(fields: "id")
              @requiresScopes(scopes: [["account:details"]]) {
              id: ID
              details: AccountDetail @external
              detailsSummary: String
                @requires(
                  fields: "details { summary ... on ScopedAccountDetail { sensitiveLevel } ... on PublicAccountDetail { publicNote } ... on AdminAccountDetail { adminLevel } }"
                )
                @requiresScopes(
                  scopes: [
                    [
                      "scoped:summary"
                      "admin:summary"
                      "scoped:level"
                      "public:note"
                      "admin:level"
                    ]
                  ]
                )
            }

            interface AccountDetail {
              summary: String
            }

            type ScopedAccountDetail implements AccountDetail @external {
              summary: String
              sensitiveLevel: Int
            }

            type PublicAccountDetail implements AccountDetail @external {
              summary: String
              publicNote: String
            }

            type AdminAccountDetail implements AccountDetail @external {
              summary: String
              adminLevel: Int
            }
          "#)),
                service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.9"
                import: ["@requiresScopes", "@key"]
              )

            type Account @key(fields: "id") {
              id: ID
              details: AccountDetail
                @requiresScopes(scopes: [["account:details"]])
            }

            interface AccountDetail {
              summary: String
            }

            type ScopedAccountDetail implements AccountDetail {
              summary: String @requiresScopes(scopes: [["scoped:summary"]])
              sensitiveLevel: Int @requiresScopes(scopes: [["scoped:level"]])
            }

            type PublicAccountDetail implements AccountDetail {
              summary: String
              publicNote: String @requiresScopes(scopes: [["public:note"]])
            }

            type AdminAccountDetail implements AccountDetail {
              summary: String @requiresScopes(scopes: [["admin:summary"]])
              adminLevel: Int @requiresScopes(scopes: [["admin:level"]])
            }
          "#)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type Account
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id")
          @requiresScopes(scopes: [["account:details"]]) {
          id: ID
          details: AccountDetail
            @join__field(graph: A, external: true)
            @join__field(graph: B)
            @requiresScopes(scopes: [["account:details"]])
          detailsSummary: String
            @join__field(
              graph: A
              requires: "details { summary ... on ScopedAccountDetail { sensitiveLevel } ... on PublicAccountDetail { publicNote }  ... on AdminAccountDetail { adminLevel } }"
            )
            @requiresScopes(
              scopes: [
                [
                  "scoped:summary"
                  "admin:summary"
                  "scoped:level"
                  "public:note"
                  "admin:level"
                ]
              ]
            )
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        interface AccountDetail @join__type(graph: A) @join__type(graph: B) {
          summary: String
            @requiresScopes(scopes: [["admin:summary", "scoped:summary"]])
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type PublicAccountDetail implements AccountDetail
          @join__implements(graph: A, interface: "AccountDetail")
          @join__implements(graph: B, interface: "AccountDetail")
          @join__type(graph: A)
          @join__type(graph: B) {
          summary: String
            @join__field(graph: A, external: true)
            @join__field(graph: B)
          publicNote: String
            @join__field(graph: A, external: true)
            @join__field(graph: B)
            @requiresScopes(scopes: [["public:note"]])
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type ScopedAccountDetail implements AccountDetail
          @join__implements(graph: A, interface: "AccountDetail")
          @join__implements(graph: B, interface: "AccountDetail")
          @join__type(graph: A)
          @join__type(graph: B) {
          summary: String
            @join__field(graph: A, external: true)
            @join__field(graph: B)
            @requiresScopes(scopes: [["scoped:summary"]])
          sensitiveLevel: Int
            @join__field(graph: A, external: true)
            @join__field(graph: B)
            @requiresScopes(scopes: [["scoped:level"]])
        }
      "#));

            to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type AdminAccountDetail implements AccountDetail
          @join__implements(graph: A, interface: "AccountDetail")
          @join__implements(graph: B, interface: "AccountDetail")
          @join__type(graph: A)
          @join__type(graph: B) {
          summary: String
            @join__field(graph: A, external: true)
            @join__field(graph: B)
            @requiresScopes(scopes: [["admin:summary"]])
          adminLevel: Int
            @join__field(graph: A, external: true)
            @join__field(graph: B)
            @requiresScopes(scopes: [["admin:level"]])
        }
      "#));
        }
    }
}
