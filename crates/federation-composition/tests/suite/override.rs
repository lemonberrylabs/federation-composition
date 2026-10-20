//! Port of `__tests__/override.spec.ts`.

use crate::shared::*;

test_versions! {
    fn non_progressive_override_with_requires_source_keeps_current_implementation_behavior(api, version) {
        let result = api.compose_services(&[
            service("original", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@requires", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @external
            paymentReceipt: String! @shareable @requires(fields: "creditCardNumber")
          }

          type Query {
            product(id: ID!): Product
          }
        "#, version)),
            service("new", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@override", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @shareable
            paymentReceipt: String! @shareable @override(from: "original")
          }
        "#, version)),
        ]);

        assert_composition_success(&result);
    }

    fn deep_requires_override_prunes_stale_join_field_and_join_type(api, version) {
        // Verifies our composition handles overriding a source field with deep
        // nested `@requires` by pruning stale source @join__ metadata.
        let result = api.compose_services(&[
            service("core", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@shareable"]
            )

          type Query {
            product(id: ID!): Product
          }

          type Product @key(fields: "id") {
            id: ID!
            creditCard: CreditCard! @shareable
          }

          type CreditCard {
            details: CreditCardDetails! @shareable
          }

          type CreditCardDetails {
            number: String! @shareable
          }
        "#, version)),
            service("original", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@requires", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCard: CreditCard! @external
            paymentReceipt: String!
              @shareable
              @requires(fields: "creditCard { details { number } }")
          }

          extend type CreditCard {
            details: CreditCardDetails! @external
          }

          extend type CreditCardDetails {
            number: String! @external
          }
        "#, version)),
            service("new", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@override", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCard: CreditCard! @shareable
            paymentReceipt: String! @shareable @override(from: "original")
          }

          type CreditCard {
            details: CreditCardDetails! @shareable
          }

          type CreditCardDetails {
            number: String! @shareable
          }
        "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Product
        @join__type(graph: CORE, key: "id")
        @join__type(graph: NEW, key: "id")
        @join__type(graph: ORIGINAL, key: "id") {
        id: ID!
        creditCard: CreditCard!
          @join__field(graph: CORE)
          @join__field(graph: NEW)
        paymentReceipt: String! @join__field(graph: NEW, override: "original")
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type CreditCard @join__type(graph: CORE) @join__type(graph: NEW) {
        details: CreditCardDetails!
          @join__field(graph: CORE)
          @join__field(graph: NEW)
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type CreditCardDetails @join__type(graph: CORE) @join__type(graph: NEW) {
        number: String! @join__field(graph: CORE) @join__field(graph: NEW)
      }
    "#);
    }

    fn nested_external_remains_when_another_requires_still_uses_it(api, version) {
        // If one requiring field is overridden but another still actively requires
        // part of the same nested path, required externals must remain.
        let result = api.compose_services(&[
            service("core", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@shareable"]
            )

          type Query {
            product(id: ID!): Product
          }

          type Product @key(fields: "id") {
            id: ID!
            creditCard: CreditCard! @shareable
          }

          type CreditCard {
            details: CreditCardDetails! @shareable
          }

          type CreditCardDetails {
            number: String! @shareable
            cvv: String! @shareable
          }
        "#, version)),
            service("original", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@requires", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCard: CreditCard! @external
            paymentReceipt: String!
              @shareable
              @requires(fields: "creditCard { details { number cvv } }")
            fraudScore: Int!
              @shareable
              @requires(fields: "creditCard { details { number } }")
          }

          extend type CreditCard {
            details: CreditCardDetails! @external
          }

          extend type CreditCardDetails {
            number: String! @external
            cvv: String! @external
          }
        "#, version)),
            service("new", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@override", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCard: CreditCard! @shareable
            paymentReceipt: String! @shareable @override(from: "original")
          }

          type CreditCard {
            details: CreditCardDetails! @shareable
          }

          type CreditCardDetails {
            number: String! @shareable
            cvv: String! @shareable
          }
        "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Product
        @join__type(graph: CORE, key: "id")
        @join__type(graph: NEW, key: "id")
        @join__type(graph: ORIGINAL, key: "id") {
        id: ID!
        creditCard: CreditCard!
          @join__field(external: true, graph: ORIGINAL)
          @join__field(graph: CORE)
          @join__field(graph: NEW)
        paymentReceipt: String! @join__field(graph: NEW, override: "original")
        fraudScore: Int!
          @join__field(
            graph: ORIGINAL
            requires: "creditCard { details { number } }"
          )
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type CreditCard
        @join__type(graph: CORE)
        @join__type(graph: NEW)
        @join__type(graph: ORIGINAL) {
        details: CreditCardDetails!
          @join__field(external: true, graph: ORIGINAL)
          @join__field(graph: CORE)
          @join__field(graph: NEW)
      }
    "#);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type CreditCardDetails
        @join__type(graph: CORE)
        @join__type(graph: NEW)
        @join__type(graph: ORIGINAL) {
        number: String!
          @join__field(external: true, graph: ORIGINAL)
          @join__field(graph: CORE)
          @join__field(graph: NEW)
        cvv: String! @join__field(graph: CORE) @join__field(graph: NEW)
      }
    "#);
    }

    fn overriding_one_source_graph_does_not_drop_external_usage_from_another_active_source(api, version) {
        // Verifies requires usage is tracked per source graph.
        // Overriding one source must not remove external metadata still required
        // by another non-overridden source field.
        let result = api.compose_services(&[
            service("core", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@shareable"]
            )

          type Query {
            product(id: ID!): Product
          }

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @shareable
          }
        "#, version)),
            service("original", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@requires", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @external
            paymentReceipt: String! @shareable @requires(fields: "creditCardNumber")
          }
        "#, version)),
            service("legacy", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@requires", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @external
            fraudScore: Int! @shareable @requires(fields: "creditCardNumber")
          }
        "#, version)),
            service("new", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@override", "@shareable"]
            )

          type Product @key(fields: "id") {
            id: ID!
            creditCardNumber: String! @shareable
            paymentReceipt: String! @shareable @override(from: "original")
          }
        "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), r#"
      type Product
        @join__type(graph: CORE, key: "id")
        @join__type(graph: LEGACY, key: "id")
        @join__type(graph: NEW, key: "id")
        @join__type(graph: ORIGINAL, key: "id") {
        id: ID!
        creditCardNumber: String!
          @join__field(graph: CORE)
          @join__field(graph: LEGACY, external: true)
          @join__field(graph: NEW)
        paymentReceipt: String! @join__field(graph: NEW, override: "original")
        fraudScore: Int!
          @join__field(graph: LEGACY, requires: "creditCardNumber")
      }
    "#);
    }
}
