//! Port of `__tests__/supergraph-composition.spec.ts`.

use crate::shared::*;

/// `expect(value).not.toEqual(expect.stringContaining(substring))`: `value` is not a string
/// that contains `substring`.
#[track_caller]
fn assert_not_string_containing(value: Option<&str>, substring: &str) {
    if let Some(value) = value {
        assert!(!value.contains(substring), "expected not to contain {substring:?}:\n{value}");
    }
}

test_implementations! {
    fn composition_of_basic_object_types(api) {
        let result = api.compose_services(&[
            service("products", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key"]
            )

          type Query {
            products: [Product!]!
          }

          type Product @key(fields: "id") {
            id: ID!
            name: String!
          }
        "#)),
            service("reviews", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key"]
            )

          type Query {
            reviews: [Review!]!
          }

          type Product @key(fields: "id") {
            id: ID!
            reviews: [Review!]!
          }

          type Review @key(fields: "id") {
            id: ID!
            body: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Product
        @join__type(graph: PRODUCTS, key: "id")
        @join__type(graph: REVIEWS, key: "id") {
        id: ID!
        name: String! @join__field(graph: PRODUCTS)
        reviews: [Review!]! @join__field(graph: REVIEWS)
      }
    "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Product {
          id: ID!
          name: String!
          reviews: [Review!]!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Review @join__type(graph: REVIEWS, key: "id") {
        id: ID!
        body: String!
      }
    "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Review {
          id: ID!
          body: String!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query @join__type(graph: PRODUCTS) @join__type(graph: REVIEWS) {
        products: [Product!]! @join__field(graph: PRODUCTS)
        reviews: [Review!]! @join__field(graph: REVIEWS)
      }
    "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Query {
          products: [Product!]!
          reviews: [Review!]!
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      enum join__Graph {
        PRODUCTS @join__graph(name: "products", url: "")
        REVIEWS @join__graph(name: "reviews", url: "")
      }
    "#);

        assert_not_string_containing(result.public_sdl(), "join__Graph");
    }

    fn composition_of_basic_object_types_with_requires_provides_key(api) {
        let result = api.compose_services(&[
            service("accounts", graphql!(r#"
          extend type Query {
            me: User
          }

          type User @key(fields: "id") {
            id: ID!
            name: String
            username: String
          }
        "#)),
            service("inventory", graphql!(r#"
          extend type Product @key(fields: "upc") {
            upc: String! @external
            weight: Int @external
            price: Int @external
            inStock: Boolean
            shippingEstimate: Int @requires(fields: "price weight")
          }
        "#)),
            service("products", graphql!(r#"
          extend type Query {
            topProducts(first: Int = 5): [Product]
          }

          type Product @key(fields: "upc") {
            upc: String!
            name: String
            price: Int
            weight: Int
          }
        "#)),
            service("reviews", graphql!(r#"
          type Review @key(fields: "id") {
            id: ID!
            body: String
            author: User @provides(fields: "username")
            product: Product
          }

          extend type User @key(fields: "id") {
            id: ID! @external
            username: String @external
            reviews: [Review]
          }

          extend type Product @key(fields: "upc") {
            upc: String! @external
            reviews: [Review]
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Product
        @join__type(graph: INVENTORY, key: "upc")
        @join__type(graph: PRODUCTS, key: "upc")
        @join__type(graph: REVIEWS, key: "upc") {
        upc: String!
        weight: Int
          @join__field(graph: INVENTORY, external: true)
          @join__field(graph: PRODUCTS)
        price: Int
          @join__field(graph: INVENTORY, external: true)
          @join__field(graph: PRODUCTS)
        inStock: Boolean @join__field(graph: INVENTORY)
        shippingEstimate: Int
          @join__field(graph: INVENTORY, requires: "price weight")
        name: String @join__field(graph: PRODUCTS)
        reviews: [Review] @join__field(graph: REVIEWS)
      }
    "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Product {
          upc: String!
          weight: Int
          price: Int
          inStock: Boolean
          shippingEstimate: Int
          name: String
          reviews: [Review]
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type User
        @join__type(graph: ACCOUNTS, key: "id")
        @join__type(graph: REVIEWS, key: "id") {
        id: ID!
        name: String @join__field(graph: ACCOUNTS)
        username: String
          @join__field(graph: ACCOUNTS)
          @join__field(graph: REVIEWS, external: true)
        reviews: [Review] @join__field(graph: REVIEWS)
      }
    "#);

        to_contain_graphql(result.public_sdl(), r#"
        type User {
          id: ID!
          name: String
          username: String
          reviews: [Review]
        }
      "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Review @join__type(graph: REVIEWS, key: "id") {
        id: ID!
        body: String
        author: User @join__field(graph: REVIEWS, provides: "username")
        product: Product
      }
    "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Review {
          id: ID!
          body: String
          author: User
          product: Product
        }
      "#);
    }

    fn fed_v1_set_join_type_extension_true_to_types_with_extends(api) {
        let result = api.compose_services(&[
            service("foo", graphql!(r#"
          type Action @extends @key(fields: "id") {
            id: ID! @external
            words: [String]!
          }
        "#)),
            service("bar", graphql!(r#"
          type Action @key(fields: "id") {
            id: ID!
            name: String!
          }

          type Query {
            actions: [Action!]!
          }
        "#)),
            service("baz", graphql!(r#"
          type Action @extends @key(fields: "id") {
            id: ID! @external
            photos: [String]!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Action
        @join__type(graph: BAR, key: "id")
        @join__type(graph: BAZ, key: "id", extension: true)
        @join__type(graph: FOO, key: "id", extension: true) {
        id: ID!
        name: String! @join__field(graph: BAR)
        photos: [String]! @join__field(graph: BAZ)
        words: [String]! @join__field(graph: FOO)
      }
    "#);
    }

    fn fed_v2_dgs_set_join_type_override_requires_to_types_with_override_and_requires(api) {
        let result = api.compose_services(&[
            service("foo", graphql!(r#"
          schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable", "@inaccessible"]
            ) {
            query: Query
          }

          scalar federation__FieldSet

          scalar link__Import

          enum link__Purpose {
            EXECUTION
            SECURITY
          }

          directive @link(
            as: String
            for: link__Purpose
            import: [link__Import]
            url: String
          ) repeatable on SCHEMA

          directive @key(
            fields: federation__FieldSet!
            resolvable: Boolean = true
          ) repeatable on INTERFACE | OBJECT
          directive @inaccessible on ARGUMENT_DEFINITION | ENUM | ENUM_VALUE | FIELD_DEFINITION | INPUT_FIELD_DEFINITION | INPUT_OBJECT | INTERFACE | OBJECT | SCALAR | UNION
          directive @shareable on FIELD_DEFINITION | OBJECT

          type Product @key(fields: "id") {
            id: ID!
            variants: [Variant!]! @shareable
            price: Int
            colors: [Color!]! @inaccessible
          }

          type Color @key(fields: "id") {
            id: ID!
          }

          type Variant @key(fields: "id") {
            id: ID!
          }

          type Query {
            word: String
          }
        "#)),
            service("bar", graphql!(r#"
          schema
            @link(url: "https://specs.apollo.dev/link/v1.0")
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: [
                "@key"
                "@external"
                "@requires"
                "@shareable"
                "@override"
              ]
            ) {
            query: Query
          }

          scalar federation__FieldSet

          scalar link__Import

          enum link__Purpose {
            EXECUTION
            SECURITY
          }

          directive @external(reason: String) on FIELD_DEFINITION | OBJECT
          directive @key(
            fields: federation__FieldSet!
            resolvable: Boolean = true
          ) repeatable on INTERFACE | OBJECT
          directive @link(
            as: String
            for: link__Purpose
            import: [link__Import]
            url: String
          ) repeatable on SCHEMA
          directive @override(from: String!) on FIELD_DEFINITION
          directive @requires(fields: federation__FieldSet!) on FIELD_DEFINITION
          directive @shareable on FIELD_DEFINITION | OBJECT

          type Product @key(fields: "id") {
            id: ID!
            variants: [Variant!]!
              @override(from: "foo")
              @requires(fields: "colors { id }")
            colors: [Color!]! @external
          }

          type Color @key(fields: "id") {
            id: ID!
          }

          type Variant @key(fields: "id") {
            id: ID!
          }

          type Query {
            words: [String!]!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Product
        @join__type(graph: BAR, key: "id")
        @join__type(graph: FOO, key: "id") {
        id: ID!
        variants: [Variant!]!
          @join__field(graph: BAR, requires: "colors { id }", override: "foo")
        colors: [Color!]!
          @inaccessible
          @join__field(graph: BAR, external: true)
          @join__field(graph: FOO)
        price: Int @join__field(graph: FOO)
      }
    "#);

        to_contain_graphql(result.public_sdl(), r#"
        type Product {
          id: ID!
          variants: [Variant!]!
          price: Int
        }
      "#);
    }

    fn fed_v1_override(api) {
        let result = api.compose_services(&[
            service("foo", graphql!(r#"
          directive @override(from: String!) on FIELD_DEFINITION

          type Sentence {
            text: String! @override(from: "bar")
            color: String! @override(from: "bar")
          }

          type Viewer {
            sentences: [Sentence!]! @override(from: "bar")
          }

          type Query {
            sentences: [Sentence!]! @override(from: "bar")
            view: Viewer!
          }
        "#)),
            service("bar", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@shareable"]
            )

          type Sentence {
            text: String!
            color: String!
          }

          type Viewer @shareable {
            sentences: [Sentence!]!
            words: [String!]!
          }

          type Query @shareable {
            sentences: [Sentence!]!
            view: Viewer!
          }
        "#)),
            service("baz", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@shareable"]
            )

          type Viewer @shareable {
            ping: String
          }

          type Query @shareable {
            view: Viewer!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Sentence @join__type(graph: BAR) @join__type(graph: FOO) {
        color: String! @join__field(graph: FOO, override: "bar")
        text: String! @join__field(graph: FOO, override: "bar")
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Viewer
        @join__type(graph: BAR)
        @join__type(graph: FOO)
        @join__type(graph: BAZ) {
        sentences: [Sentence!]! @join__field(graph: FOO, override: "bar")
        words: [String!]! @join__field(graph: BAR)
        ping: String @join__field(graph: BAZ)
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query
        @join__type(graph: BAR)
        @join__type(graph: FOO)
        @join__type(graph: BAZ) {
        sentences: [Sentence!]! @join__field(graph: FOO, override: "bar")
        view: Viewer!
      }
    "#);
    }

    #[ignore = "skipped in TypeScript"]
    fn validate_fixtures_huge_schema(api) {
        let subgraphs = get_subgraphs_of_huge_schema();
        let result = api.compose_services(&subgraphs);
        assert_composition_success(&result);
    }

    fn validate_fixtures_dgs(api) {
        let subgraphs = get_subgraphs_of_dgs();
        let result = api.compose_services(&subgraphs);
        assert_composition_success(&result);
    }

    fn validate_authentication(api) {
        let result = api.compose_services(&[
            service("feed", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.6"
              import: ["@requiresScopes", "@policy", "@key", "@shareable"]
            )

          type Query {
            feed: [Post!]!
              @shareable
              @requiresScopes(scopes: [["read:posts"]])
              @policy(policies: [["read_posts"]])
          }

          type Post @key(fields: "id") {
            id: ID!
            title: String!
          }
        "#)),
            service("comments", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.6"
              import: [
                "@authenticated"
                "@requiresScopes"
                "@policy"
                "@key"
                "@external"
                "@shareable"
              ]
            )

          extend type Post @key(fields: "id") {
            id: ID! @external
            comments: [Comment!]! @policy(policies: [["read_post_comments"]])
          }

          extend type Query {
            feed: [Post!]!
              @shareable
              @policy(
                policies: [["read_post_comments"], ["read_post_comments"]]
              )
              @requiresScopes(scopes: [["read:posts"]])
          }

          type Mutation {
            commentPost(input: CommentPostInput!): Comment! @authenticated
          }

          input CommentPostInput {
            postId: ID!
            comment: String!
          }

          type Comment {
            id: ID!
            body: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION)
        @link(url: "https://specs.apollo.dev/authenticated/v0.1", for: SECURITY)
        @link(url: "https://specs.apollo.dev/policy/v0.1", for: SECURITY)
        @link(
          url: "https://specs.apollo.dev/requiresScopes/v0.1"
          for: SECURITY
        ) {
        query: Query
        mutation: Mutation
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      directive @authenticated on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
    "#);

        assert_not_to_match_string(result.public_sdl(), "@authenticated");

        to_contain_graphql(result.supergraph_sdl(), r#"
      directive @requiresScopes(
        scopes: [[requiresScopes__Scope!]!]!
      ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
    "#);

        assert_not_to_match_string(result.public_sdl(), "@requiresScopes");

        to_contain_graphql(result.supergraph_sdl(), r#"
      scalar requiresScopes__Scope
    "#);

        assert_not_to_match_string(result.public_sdl(), "requiresScopes__Scope");

        to_contain_graphql(result.supergraph_sdl(), r#"
      directive @policy(
        policies: [[policy__Policy!]!]!
      ) on ENUM | FIELD_DEFINITION | INTERFACE | OBJECT | SCALAR
    "#);

        assert_not_to_match_string(result.public_sdl(), "policy");

        to_contain_graphql(result.supergraph_sdl(), r#"
      scalar policy__Policy
    "#);

        assert_not_to_match_string(result.public_sdl(), "policy__Policy");

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Mutation @join__type(graph: COMMENTS) {
        commentPost(input: CommentPostInput!): Comment! @authenticated
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query @join__type(graph: COMMENTS) @join__type(graph: FEED) {
        feed: [Post!]!
          @policy(policies: [["read_post_comments", "read_posts"]])
          @requiresScopes(scopes: [["read:posts"]])
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Post
        @join__type(extension: true, graph: COMMENTS, key: "id")
        @join__type(graph: FEED, key: "id") {
        comments: [Comment!]!
          @join__field(graph: COMMENTS)
          @policy(policies: [["read_post_comments"]])
        id: ID!
        title: String! @join__field(graph: FEED)
      }
    "#);
    }

    fn removes_federation_policy_and_federation_scope_scalars(api) {
        let result = api.compose_services(&[
            service("feed", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.6"
              import: ["@requiresScopes", "@policy", "@key", "@shareable"]
            )

          scalar federation__Scope
          scalar federation__Policy

          directive @federation__requiresScopes(
            scopes: [[federation__Scope!]!]!
          ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
          directive @federation__policy(
            policies: [[federation__Policy!]!]!
          ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM

          type Query {
            feed: [Post!]!
              @shareable
              @requiresScopes(scopes: [["read:posts"]])
              @policy(policies: [["read_posts"]])
          }

          type Post @key(fields: "id") {
            id: ID!
            title: String!
          }
        "#)),
            service("comments", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.6"
              import: [
                "@authenticated"
                "@requiresScopes"
                "@policy"
                "@key"
                "@external"
                "@shareable"
              ]
            )

          scalar federation__Scope
          scalar federation__Policy

          directive @federation__requiresScopes(
            scopes: [[federation__Scope!]!]!
          ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM
          directive @federation__policy(
            policies: [[federation__Policy!]!]!
          ) on FIELD_DEFINITION | OBJECT | INTERFACE | SCALAR | ENUM

          extend type Post @key(fields: "id") {
            id: ID! @external
            comments: [Comment!]! @policy(policies: [["read_post_comments"]])
          }

          extend type Query {
            feed: [Post!]!
              @shareable
              @policy(
                policies: [["read_post_comments"], ["read_post_comments"]]
              )
              @requiresScopes(scopes: [["read:posts"]])
          }

          type Mutation {
            commentPost(input: CommentPostInput!): Comment! @authenticated
          }

          input CommentPostInput {
            postId: ID!
            comment: String!
          }

          type Comment {
            id: ID!
            body: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        assert_not_to_match_string(result.supergraph_sdl(), "federation__Policy");
        assert_not_to_match_string(result.supergraph_sdl(), "federation__Scope");
    }

    fn override_with_shareable_in_different_conditions(api) {
        let mut result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable", "@override"]
            )

          type Query {
            bar: String! @shareable @override(from: "a")
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query
        @join__type(graph: A)
        @join__type(graph: B)
        @join__type(graph: C) {
        bar: String!
          @join__field(graph: B)
          @join__field(graph: C, override: "a")
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable", "@override"]
            )

          type Query {
            bar: String! @shareable @override(from: "a")
          }
        "#)),
            service("d", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            foo: String!
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query
        @join__type(graph: A)
        @join__type(graph: B)
        @join__type(graph: C)
        @join__type(graph: D) {
        foo: String! @join__field(graph: D)
        bar: String!
          @join__field(graph: B)
          @join__field(graph: C, override: "a")
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query
        @join__type(graph: A)
        @join__type(graph: B)
        @join__type(graph: C) {
        bar: String!
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            bar: String! @shareable
          }
        "#)),
            service("d", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable"]
            )

          type Query {
            foo: String! @shareable
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query
        @join__type(graph: A)
        @join__type(graph: B)
        @join__type(graph: C)
        @join__type(graph: D) {
        foo: String! @join__field(graph: D)
        bar: String!
          @join__field(graph: A)
          @join__field(graph: B)
          @join__field(graph: C)
      }
    "#);

        result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@shareable", "@override"]
            )

          type Query {
            bar: String! @shareable @override(from: "non-existent")
          }
        "#)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Query @join__type(graph: A) {
        bar: String!
      }
    "#);
    }
}
