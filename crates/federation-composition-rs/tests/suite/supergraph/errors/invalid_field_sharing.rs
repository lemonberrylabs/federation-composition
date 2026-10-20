//! Port of `__tests__/supergraph/errors/INVALID_FIELD_SHARING.spec.ts`.

use crate::shared::*;

// TS: the spec file's top-level `describe` block has the file's name.
#[allow(clippy::module_inception)]
mod invalid_field_sharing {
    use super::*;

    test_versions! {
        fn set_of_tests(api, version) {
            assert_errors_contain(
                api.compose_services(&[
                    service("users", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@shareable"]
                )

              type Query {
                users: [User]
              }

              type User @key(fields: "id") {
                id: ID
                profile: Profile
              }

              type Profile @shareable {
                name: String
              }
            "#, version)),
                    service("feed", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@shareable"]
                )

              type User @key(fields: "id") {
                id: ID
                profile: Profile
              }

              type Profile @shareable {
                name: String
              }
            "#, version)),
                ]),
                &[
                    ErrorMatcher::new()
                        .message_containing(r#"Non-shareable field "User.profile" is resolved from multiple subgraphs: it is resolved from subgraphs "feed" and "users" and defined as non-shareable in all of them"#)
                        .code("INVALID_FIELD_SHARING"),
                ],
            );

            assert_errors_contain(
                api.compose_services(&[
                    service("users", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key"]
                )

              type Query {
                user: User
              }

              type User {
                id: ID!
                name: String
              }
            "#, version)),
                    service("feed", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@shareable"]
                )

                type User @key(fields: "id") {
                  id: ID!
                  comments: [String]
                }

                type Query {
                  users: [User]
                }
            "#, version)),
                ]),
                &[
                    ErrorMatcher::new()
                        .message_containing(r#"Non-shareable field "User.id" is resolved from multiple subgraphs: it is resolved from subgraphs "feed" and "users" and defined as non-shareable in subgraph "users""#)
                        .code("INVALID_FIELD_SHARING"),
                ],
            );

            assert_errors_contain(
                api.compose_services(&[
                    service("users", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@shareable"]
                )

                extend type Query {
                  foo: Foo
                }

                type Foo @shareable @key(fields: "id") {
                  id: ID!
                  name: String
                }
            "#, version)),
                    service("feed", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@shareable", "@override"]
                )

                extend type Query {
                  foo: Foo @override(from: "noop")
                }

                type Foo @shareable @key(fields: "id") {
                  id: ID!
                  name: String
                }
            "#, version)),
                    service("noop", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@shareable"]
                )

                type Query {
                  noop: String
                }
            "#, version)),
                ]),
                &[
                    ErrorMatcher::new()
                        .message_containing(r#"Non-shareable field "Query.foo" is resolved from multiple subgraphs: it is resolved from subgraphs "feed" and "users" and defined as non-shareable in all of them"#)
                        .code("INVALID_FIELD_SHARING"),
                ],
            );

            assert_errors_contain(
                api.compose_services(&[
                    service("foo", graphql!(r#"
              extend schema
              @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@shareable"]
                )

              extend type Note {
                url: String!
              }

              type Note {
                name: String!
              }

              type Query {
                foo: String! @shareable
              }
            "#, version)),
                    service("bar", graphql!(r#"
              extend schema
              @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@shareable"]
                )

              extend type Note {
                url: String!
              }

              type Note {
                name: String!
              }

              type Query {
                bar: String! @shareable
              }
            "#, version)),
                ]),
                &[
                    ErrorMatcher::new()
                        .message_containing(r#"Non-shareable field "Note.url" is resolved from multiple subgraphs: it is resolved from subgraphs "bar" and "foo" and defined as non-shareable in all of them"#)
                        .code("INVALID_FIELD_SHARING"),
                ],
            );
        }

        fn fed_v1(api, version) {
            assert_errors_contain(
                api.compose_services(&[
                    service("foo", graphql!(r#"
              extend type Note {
                url: String!
              }

              type Note {
                name: String!
              }

              type Query {
                foo: String!
              }
            "#)),
                    service("bar", graphql!(r#"
              extend type Note {
                url: String!
              }

              type Note {
                name: String!
              }

              type Query {
                bar: String!
              }
            "#)),
                ]),
                &[
                    ErrorMatcher::new()
                        .message_containing(r#"Non-shareable field "Note.url" is resolved from multiple subgraphs: it is resolved from subgraphs "bar" and "foo" and defined as non-shareable in all of them"#)
                        .code("INVALID_FIELD_SHARING"),
                ],
            );

            assert_composition_success(&api.compose_services(&[
                service("foo", graphql!(r#"
              extend type Note @key(fields: "id") {
                id: ID!
                url: String!
              }

              type Note {
                name: String!
              }

              type Query {
                foo: String!
              }
            "#)),
                service("bar", graphql!(r#"
              extend type Note @key(fields: "id") {
                id: ID!
                url: String!
              }

              type Note {
                name: String!
              }

              type Query {
                bar: String!
              }
            "#)),
            ]));
        }

        fn subscription_fields_fed_v1(api, version) {
            assert_errors_contain(
                api.compose_services(&[
                    service("foo", graphql!(r#"
              type Query {
                foo: String
              }

              type Subscription {
                event: String
              }
            "#)),
                    service("bar", graphql!(r#"
              type Query {
                bar: String
              }

              type Subscription {
                event: String
              }
            "#)),
                ]),
                &[
                    ErrorMatcher::new()
                        .message_containing("Fields on root level subscription object cannot be marked as shareable")
                        .code("INVALID_FIELD_SHARING"),
                ],
            );
        }

        fn subscription_fields(api, version) {
            assert_errors_contain(
                api.compose_services(&[
                    service("foo", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@shareable"]
                )

              type Query {
                foo: String
              }

              type Subscription {
                event: String @shareable
              }
            "#, version)),
                    service("bar", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@shareable"]
                )

              type Query {
                bar: String
              }

              type Subscription {
                event: String @shareable
              }
            "#, version)),
                ]),
                &[
                    ErrorMatcher::new()
                        .message_containing("Fields on root level subscription object cannot be marked as shareable")
                        .code("INVALID_FIELD_SHARING"),
                ],
            );
        }

        fn nested_key_fields(api, version) {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
            extend schema
              @link(
              url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key"]
              )

            type A
              @key(fields: "id", resolvable: true)
              @key(fields: "pId", resolvable: false)
              @key(fields: "compositeId { one two }", resolvable: false)
              @key(fields: "id compositeId { two three }", resolvable: false) {
              id: ID!
              pId: ID!
              compositeId: CompositeID!
              name: String!
            }

            type CompositeID {
              one: ID!
              two: ID!
              three: ID!
            }
          "#, version)),
                service("b", graphql!(r#"
            extend schema
              @link(
              url: "https://specs.apollo.dev/federation/${version}"
                import: ["@key", "@requires", "@external"]
              )

            type Query {
              b: B
            }

            type B @key(fields: "id") {
              id: ID!
              a: [A!]!
            }

            type A
              @key(fields: "compositeId { one two }", resolvable: false)
              @key(fields: "id compositeId { two three }", resolvable: true)
              @key(fields: "pId", resolvable: false)
              @key(fields: "id", resolvable: false) {
              id: ID!
              pId: ID!
              compositeId: CompositeID!
              name: String! @external
              nameInB: String! @requires(fields: "name")
            }

            type CompositeID {
              one: ID!
              two: ID!
              three: ID!
            }
          "#, version)),
            ]);

            assert_composition_success(&result);
        }
    }
}

test_versions! {
    fn non_shareable_field_in_interface_object_and_interface_implementation(api, version) {
        let result = api.compose_services(&[
            service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@interfaceObject", "@shareable"]
            )

          type Query {
            hello: String
          }

          interface MyInterface @key(fields: "id") {
            id: ID!
            field: String
          }

          type MyType implements MyInterface @key(fields: "id") {
            id: ID!
            field: String
          }
        "#)),
            service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@interfaceObject"]
            )

          type MyInterface @key(fields: "id") @interfaceObject {
            id: ID!
            newField: String
          }
        "#)),
            service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@interfaceObject"]
            )

          type MyInterface
            @key(fields: "id", resolvable: false)
            @interfaceObject {
            id: ID!
            field: String
          }
        "#)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new()
                .message(r#"Non-shareable field "MyType.field" is resolved from multiple subgraphs: it is resolved from subgraphs "a" and "c" (through @interfaceObject field "MyInterface.field") and defined as non-shareable in all of them"#)
                .code("INVALID_FIELD_SHARING")],
        );
    }
}
