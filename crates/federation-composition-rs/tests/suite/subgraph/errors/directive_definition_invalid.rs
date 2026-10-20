//! Port of `__tests__/subgraph/errors/DIRECTIVE_DEFINITION_INVALID.spec.ts`.

use crate::shared::*;

mod extends {
    use super::*;

    test_versions! {
        fn different_location_should_be_ignored_if_not_imported(api, version) {
            assert_composition_success(&api.compose_services(&[
                service("users", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

                type User @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  users: [User]
                }
              "#, version)),
                service("profiles", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@external"]
                  )

                directive @extends on FIELD_DEFINITION

                type User @key(fields: "id") {
                  id: ID!
                  profile: Profile
                }

                type Profile {
                  name: String!
                }
              "#, version)),
            ]));
        }

        fn different_location_should_be_ignored_if_imported_but_not_used(api, version) {
            let result = api.compose_services(&[
                service("users", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

                type User @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  users: [User]
                }
              "#, version)),
                service("profiles", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@external", "@extends"]
                  )

                directive @extends on FIELD_DEFINITION

                type User @key(fields: "id") {
                  id: ID!
                  profile: Profile
                }

                type Profile {
                  name: String!
                }
              "#, version)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(
                        r#"[profiles] Invalid definition for directive "@extends": "@extends" should have locations OBJECT, INTERFACE, but found (non-subset) FIELD_DEFINITION"#,
                    )
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }
    }
}

mod external {
    use super::*;

    test_versions! {
        fn exact_same_definition_without_an_import_should_be_ignored(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

                directive @external on OBJECT | FIELD_DEFINITION

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") {
                  id: ID!
                  name: String! @external
                }
              "#, version))]));
        }

        fn exact_same_definition_with_an_import(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@external"]
                  )

                directive @external on OBJECT | FIELD_DEFINITION

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") {
                  id: ID!
                  name: String! @external
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Field "User.name" is marked @external but is not used in any federation directive (@key, @provides, @requires) or to satisfy an interface; the field declaration has no use and should be removed (or the field should not be @external)."#,
                    )
                    .code("EXTERNAL_UNUSED")],
            );
        }

        fn no_object(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@external"]
                  )

                directive @external on FIELD_DEFINITION

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") {
                  id: ID!
                  name: String! @external
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Field "User.name" is marked @external but is not used in any federation directive (@key, @provides, @requires) or to satisfy an interface; the field declaration has no use and should be removed (or the field should not be @external)."#,
                    )
                    .code("EXTERNAL_UNUSED")],
            );
        }

        fn no_field_definition(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@external"]
                  )

                directive @external on OBJECT

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") @external {
                  id: ID!
                  name: String!
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Field "User.name" is marked @external but is not used in any federation directive (@key, @provides, @requires) or to satisfy an interface; the field declaration has no use and should be removed (or the field should not be @external)."#,
                    )
                    .code("EXTERNAL_UNUSED")],
            );
        }
    }
}

mod tag {
    use super::*;

    test_versions! {
        fn exact_same_definition_should_be_ignored_if_not_imported(api, version) {
            assert_no_error_matches(
                &api.compose_services(&[
                    service("users", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@inaccessible", "@requires", "@provides", "@external"]
                  )

                directive @tag(
                  name: String!
                ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible @external @tag(name: "public")
                  profile: Profile @requires(fields: "internalId")
                }

                type Profile {
                  name: String
                }
              "#, version)),
                    service("ext", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@inaccessible", "@external"]
                  )

                type Query {
                  users: [User]
                }

                extend type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible
                }
              "#, version)),
                ]),
                &ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot apply merged directive @tag(name: "public") to external field "User.internalId""#)
                    .code("MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL"),
            );
        }

        fn exact_same_definition_with_import(api, version) {
            assert_errors_contain(
                &api.compose_services(&[
                    service("users", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: [
                      "@key"
                      "@inaccessible"
                      "@requires"
                      "@provides"
                      "@external"
                      "@tag"
                    ]
                  )

                directive @tag(
                  name: String!
                ) repeatable on FIELD_DEFINITION | INTERFACE | OBJECT | UNION | ARGUMENT_DEFINITION | SCALAR | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible @external @tag(name: "public")
                  profile: Profile @requires(fields: "internalId")
                }

                type Profile {
                  name: String
                }
              "#, version)),
                    service("ext", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@inaccessible", "@external"]
                  )

                type Query {
                  users: [User]
                }

                extend type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible
                }
              "#, version)),
                ]),
                &[ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot apply merged directive @tag(name: "public") to external field "User.internalId""#)
                    .code("MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL")],
            );
        }
    }
}

mod inaccessible {
    use super::*;

    test_versions! {
        fn exact_same_definition_and_imported(api, version) {
            assert_errors_contain(
                &api.compose_services(&[
                    service("users", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@inaccessible", "@requires", "@provides", "@external"]
                  )

                directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible @external
                  profile: Profile @requires(fields: "internalId")
                }

                type Profile {
                  name: String
                }
              "#, version)),
                    service("ext", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@inaccessible", "@external"]
                  )

                type Query {
                  users: [User]
                }

                extend type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible
                }
              "#, version)),
                ]),
                &[ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot apply merged directive @inaccessible to external field "User.internalId""#)
                    .code("MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL")],
            );
        }

        fn exact_same_definition_should_be_ignored_when_not_imported(api, version) {
            assert_no_error_matches(
                &api.compose_services(&[
                    service("users", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@requires", "@provides", "@external"]
                  )

                directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

                type Query {
                  users: [User]
                }

                type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible @external
                  profile: Profile @requires(fields: "internalId")
                }

                type Profile {
                  name: String
                }
              "#, version)),
                    service("ext", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@external"]
                  )

                directive @inaccessible on FIELD_DEFINITION | OBJECT | INTERFACE | UNION | ENUM | ENUM_VALUE | SCALAR | INPUT_OBJECT | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

                type Query {
                  users: [User]
                }

                extend type User @key(fields: "id") {
                  id: ID!
                  internalId: ID! @inaccessible
                }
              "#, version)),
                ]),
                &ErrorMatcher::new()
                    .message_containing(r#"[users] Cannot apply merged directive @inaccessible to external field "User.internalId""#)
                    .code("MERGED_DIRECTIVE_APPLICATION_ON_EXTERNAL"),
            );
        }
    }
}

mod r#override {
    use super::*;

    test_versions! {
        fn exact_same_definition_and_imported(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("billing", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@override"]
                  )

                directive @override(from: String!) on FIELD_DEFINITION

                type Query {
                  bills: [Bill]
                }

                type Bill @key(fields: "id") {
                  id: ID!
                  amount: Int! @override(from: "billing")
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(r#"Source and destination subgraphs "billing" are the same for overridden field "Bill.amount""#)
                    .code("OVERRIDE_FROM_SELF_ERROR")],
            );
        }

        fn exact_same_definition_should_be_ignored_if_not_imported(api, version) {
            assert_no_error_matches(
                &api.compose_services(&[service("billing", graphql!(r#"
                extend schema
                  @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

                directive @override(from: String!) on FIELD_DEFINITION

                type Query {
                  bills: [Bill]
                }

                type Bill @key(fields: "id") {
                  id: ID!
                  amount: Int! @override(from: "billing")
                }
              "#, version))]),
                &ErrorMatcher::new()
                    .message_containing(r#"Source and destination subgraphs "billing" are the same for overridden field "Bill.amount""#)
                    .code("OVERRIDE_FROM_SELF_ERROR"),
            );
        }
    }
}

mod key {
    use super::*;

    test_versions! {
        fn exact_same_definition_should_be_ignored_if_not_imported(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@external", "FieldSet"]
                  ) {
                  query: Query
                }

                directive @key(
                  fields: FieldSet!
                  resolvable: Boolean = true
                ) repeatable on OBJECT | INTERFACE

                scalar FieldSet

                type User @key(fields: "THERE IS NO FIELD NAMED LIKE THIS") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]));
        }

        fn fields_of_string_type(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@external", "@key"]
                  ) {
                  query: Query
                }

                directive @key(
                  fields: String!
                  resolvable: Boolean = true
                ) repeatable on OBJECT | INTERFACE

                type User @key(fields: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]));
        }

        fn fields_of_different_type(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@external", "@key"]
                  ) {
                  query: Query
                }

                directive @key(
                  fields: [String!]!
                  resolvable: Boolean = true
                ) repeatable on OBJECT | INTERFACE

                type User @key(fields: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]));

            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@external", "@key"]
                  ) {
                  query: Query
                }

                directive @key(
                  fields: Str!
                  resolvable: Boolean = true
                ) repeatable on OBJECT | INTERFACE

                scalar Str

                type User @key(fields: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Invalid definition for directive "@key": argument "fields" should have type "FieldSet!" but found type "Str!""#,
                    )
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }

        fn federation_field_set_should_be_accepted(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
          schema @link(url: "https://specs.apollo.dev/link/v1.0") {
          query: Query
        }

        extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable", "@external", "@provides"])

        directive @key(fields: federation__FieldSet!, resolvable: Boolean = true) repeatable on OBJECT | INTERFACE

        scalar federation__FieldSet

        type User @key(fields: "id") {
          id: ID!
          name: String
        }

        type Query {
          users: [User]
        }

        "#, version))]));
        }

        fn fed_v1_fields_of_string_type_should_be_accepted(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
              directive @key(fields: [String!]!) on OBJECT

              type User @key(fields: "id") {
                id: ID!
                name: String
              }

              type Query {
                users: [User]
              }
            "#))]));
        }

        fn missing_default_value_for_resolvable(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@external", "@key", "FieldSet"]
                  ) {
                  query: Query
                }

                directive @key(
                  fields: FieldSet!
                  resolvable: Boolean
                ) repeatable on OBJECT | INTERFACE

                scalar FieldSet

                type User @key(fields: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Invalid definition for directive "@key": argument "resolvable" should have default value true but found default value null"#,
                    )
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }

        fn missing_resolvable(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "FieldSet"]
                  ) {
                  query: Query
                }
                directive @key(fields: FieldSet!) repeatable on OBJECT | INTERFACE

                scalar FieldSet

                type User @key(fields: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]));
        }

        fn missing_fields(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "FieldSet"]
                  ) {
                  query: Query
                }
                directive @key(field: FieldSet!) repeatable on OBJECT | INTERFACE

                scalar FieldSet

                type User @key(field: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(r#"[users] Invalid definition for directive "@key": missing required argument "fields""#)
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }

        fn missing_repeatable(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "FieldSet"]
                  ) {
                  query: Query
                }

                directive @key(
                  fields: FieldSet!
                  resolvable: Boolean = true
                ) on OBJECT | INTERFACE

                scalar FieldSet

                type User @key(fields: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]));
        }

        fn not_on_interface(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "FieldSet"]
                  ) {
                  query: Query
                }
                directive @key(fields: FieldSet!, resolvable: Boolean = true) repeatable on OBJECT

                scalar FieldSet

                type User @key(fields: "id") {
                  id: ID!
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]));
        }

        fn not_on_object(api, version) {
            assert_composition_success(&api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "FieldSet"]
                  ) {
                  query: Query
                }
                directive @key(
                  fields: FieldSet!
                  resolvable: Boolean = true
                ) repeatable on INTERFACE

                scalar FieldSet

                type Query {
                  users: [String]
                }
              "#, version))]));
        }

        fn on_field(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
                schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "FieldSet"]
                  ) {
                  query: Query
                }
                directive @key(fields: FieldSet!, resolvable: Boolean = true) repeatable on FIELD

                scalar FieldSet

                type User {
                  id: ID! @key(fields: "id")
                  name: String
                }

                type Query {
                  users: [User]
                }
              "#, version))]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Invalid definition for directive "@key": "@key" should have locations OBJECT, INTERFACE, but found (non-subset) FIELD"#,
                    )
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }

        fn fed_v1_uses_field_set(api, version) {
            assert_errors_contain(
                &api.compose_services(&[service("users", graphql!(r#"
              scalar FieldSet
              directive @key(
                fields: FieldSet!
                resolvable: Boolean = true
              ) repeatable on OBJECT | INTERFACE

              type User @key(fields: "id") {
                id: ID!
                name: String
              }

              type Query {
                users: [User]
              }
            "#))]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[users] Invalid definition for directive "@key": argument "fields" should have type "_FieldSet!" but found type "FieldSet!""#,
                    )
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }
    }
}

mod requires {
    use super::*;

    test_versions! {
        fn should_be_ignored_if_not_imported(api, version) {
            let stars_stuff = create_stars_stuff();

            assert_composition_success(&api.compose_services(&[
                stars_stuff.users,
                stars_stuff.pandas,
                stars_stuff.products,
                stars_stuff.reviews,
                service("inventory", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@shareable", "@external", "FieldSet"]
                )

              directive @requires(fields: FieldSet!) on FIELD_DEFINITION
              scalar FieldSet

              type Product implements ProductItf @key(fields: "id") {
                id: ID!
                dimensions: ProductDimension @external
                delivery(zip: String): DeliveryEstimates @requires(fields: "IT DOES NOT EXIST!!!!! ")
              }

              type ProductDimension @shareable {
                size: String
                weight: Float
              }

              type DeliveryEstimates {
                estimatedDelivery: String
                fastestDelivery: String
              }

              interface ProductItf {
                id: ID!
                dimensions: ProductDimension
                delivery(zip: String): DeliveryEstimates
              }

              enum ShippingClass {
                STANDARD
                EXPRESS
                OVERNIGHT
              }
            "#, version)),
            ]));
        }

        fn exact_definition(api, version) {
            let stars_stuff = create_stars_stuff();

            assert_composition_success(&api.compose_services(&[
                stars_stuff.users,
                stars_stuff.pandas,
                stars_stuff.products,
                stars_stuff.reviews,
                service("inventory", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@shareable", "@external", "FieldSet", "@requires"]
                  )

                directive @requires(fields: FieldSet!) on FIELD_DEFINITION
                scalar FieldSet

                type Product implements ProductItf @key(fields: "id") {
                  id: ID!
                  dimensions: ProductDimension @external
                  delivery(zip: String): DeliveryEstimates
                    @requires(fields: "dimensions { size weight }")
                }

                type ProductDimension @shareable {
                  size: String
                  weight: Float
                }

                type DeliveryEstimates {
                  estimatedDelivery: String
                  fastestDelivery: String
                }

                interface ProductItf {
                  id: ID!
                  dimensions: ProductDimension
                  delivery(zip: String): DeliveryEstimates
                }

                enum ShippingClass {
                  STANDARD
                  EXPRESS
                  OVERNIGHT
                }
              "#, version)),
            ]));
        }

        fn fields_of_string_type(api, version) {
            let stars_stuff = create_stars_stuff();

            assert_composition_success(&api.compose_services(&[
                stars_stuff.users,
                stars_stuff.pandas,
                stars_stuff.products,
                stars_stuff.reviews,
                service("inventory", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@shareable", "@external", "@requires"]
                  )

                directive @requires(fields: String!) on FIELD_DEFINITION

                type Product implements ProductItf @key(fields: "id") {
                  id: ID!
                  dimensions: ProductDimension @external
                  delivery(zip: String): DeliveryEstimates
                    @requires(fields: "dimensions { size weight }")
                }

                type ProductDimension @shareable {
                  size: String
                  weight: Float
                }

                type DeliveryEstimates {
                  estimatedDelivery: String
                  fastestDelivery: String
                }

                interface ProductItf {
                  id: ID!
                  dimensions: ProductDimension
                  delivery(zip: String): DeliveryEstimates
                }

                enum ShippingClass {
                  STANDARD
                  EXPRESS
                  OVERNIGHT
                }
              "#, version)),
            ]));
        }

        fn fields_of_different_type(api, version) {
            let stars_stuff = create_stars_stuff();

            assert_errors_contain(
                &api.compose_services(&[
                    stars_stuff.users,
                    stars_stuff.pandas,
                    stars_stuff.products,
                    stars_stuff.reviews,
                    service("inventory", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@shareable", "@external", "@requires"]
                  )

                directive @requires(fields: Str!) on FIELD_DEFINITION
                scalar Str

                type Product implements ProductItf @key(fields: "id") {
                  id: ID!
                  dimensions: ProductDimension @external
                  delivery(zip: String): DeliveryEstimates
                    @requires(fields: "dimensions { size weight }")
                }

                type ProductDimension @shareable {
                  size: String
                  weight: Float
                }

                type DeliveryEstimates {
                  estimatedDelivery: String
                  fastestDelivery: String
                }

                interface ProductItf {
                  id: ID!
                  dimensions: ProductDimension
                  delivery(zip: String): DeliveryEstimates
                }

                enum ShippingClass {
                  STANDARD
                  EXPRESS
                  OVERNIGHT
                }
              "#, version)),
                ]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[inventory] Invalid definition for directive "@requires": argument "fields" should have type "FieldSet!" but found type "Str!""#,
                    )
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }

        fn missing_fields(api, version) {
            let stars_stuff = create_stars_stuff();

            assert_errors_contain(
                &api.compose_services(&[
                    stars_stuff.users,
                    stars_stuff.pandas,
                    stars_stuff.products,
                    stars_stuff.reviews,
                    service("inventory", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@shareable", "@external", "@requires"]
                  )

                directive @requires(field: FieldSet!) on FIELD_DEFINITION
                scalar FieldSet

                type Product implements ProductItf @key(fields: "id") {
                  id: ID!
                  dimensions: ProductDimension @external
                  delivery(zip: String): DeliveryEstimates
                    @requires(field: "dimensions { size weight }")
                }

                type ProductDimension @shareable {
                  size: String
                  weight: Float
                }

                type DeliveryEstimates {
                  estimatedDelivery: String
                  fastestDelivery: String
                }

                interface ProductItf {
                  id: ID!
                  dimensions: ProductDimension
                  delivery(zip: String): DeliveryEstimates
                }

                enum ShippingClass {
                  STANDARD
                  EXPRESS
                  OVERNIGHT
                }
              "#, version)),
                ]),
                &[ErrorMatcher::new()
                    .message_containing(r#"[inventory] Invalid definition for directive "@requires": missing required argument "fields""#)
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }

        fn on_object(api, version) {
            let stars_stuff = create_stars_stuff();

            assert_errors_contain(
                &api.compose_services(&[
                    stars_stuff.users,
                    stars_stuff.pandas,
                    stars_stuff.products,
                    stars_stuff.reviews,
                    service("inventory", graphql!(r#"
                extend schema
                  @link(
                    url: "https://specs.apollo.dev/federation/${version}"
                    import: ["@key", "@shareable", "@external", "@requires", "FieldSet"]
                  )

                directive @requires(fields: FieldSet!) on OBJECT
                scalar FieldSet

                type Product implements ProductItf @key(fields: "id") {
                  id: ID!
                  dimensions: ProductDimension @external
                  delivery(zip: String): DeliveryEstimates
                    @requires(fields: "dimensions { size weight }")
                }

                type ProductDimension @shareable {
                  size: String
                  weight: Float
                }

                type DeliveryEstimates {
                  estimatedDelivery: String
                  fastestDelivery: String
                }

                interface ProductItf {
                  id: ID!
                  dimensions: ProductDimension
                  delivery(zip: String): DeliveryEstimates
                }

                enum ShippingClass {
                  STANDARD
                  EXPRESS
                  OVERNIGHT
                }
              "#, version)),
                ]),
                &[ErrorMatcher::new()
                    .message_containing(
                        r#"[inventory] Invalid definition for directive "@requires": "@requires" should have locations FIELD_DEFINITION, but found (non-subset) OBJECT"#,
                    )
                    .code("DIRECTIVE_DEFINITION_INVALID")],
            );
        }
    }
}
