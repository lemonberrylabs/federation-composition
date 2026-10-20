//! Port of `__tests__/override-with-label.spec.ts`.
//!
//! The file installs a snapshot serializer for `specs.apollo.dev` strings, but has no snapshots.

use crate::shared::*;

/// The body of `test.each([...])(\`label: "%s" should fail\`, (label) => { ... })`.
fn label_should_fail(api: Api, version: &str, label: &str) {
    let result = api.compose_services(&[
        service("foo", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override"])

          type Query {
            foo: User
          }

          type User @key(fields: "id") {
            id: ID
            name: String @override(from: "bar", label: "${label}")
          }
        "#, version, label)),
        service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              bar: User
            }

            type User @key(fields: "id") {
              id: ID
              name: String
            }
          "#, version)),
    ]);

    assert_composition_failure(&result);
    assert_errors_contain(
        result.errors(),
        &[ErrorMatcher::new()
            .message(format!(
                r#"[foo] Invalid @override label "{label}" on field "User.name" on subgraph "foo": labels must start with a letter and after that may contain alphanumerics, underscores, minuses, colons, periods, or slashes. Alternatively, labels may be of the form "percent(x)" where x is a float between 0-100 inclusive."#
            ))
            .code("OVERRIDE_LABEL_INVALID")],
    );
}

/// The body of `test.each([...])(\`label: "%s" should pass\`, (label) => { ... })`.
fn label_should_pass(api: Api, version: &str, label: &str) {
    let result = api.compose_services(&[
        service("foo", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override"])

          type Query {
            foo: User
          }

          type User @key(fields: "id") {
            id: ID
            name: String @override(from: "bar", label: "${label}")
          }
        "#, version, label)),
        service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Query {
              bar: User
            }

            type User @key(fields: "id") {
              id: ID
              name: String
            }
          "#, version)),
    ]);

    assert_composition_success(&result);
}

mod override_label {
    use super::*;

    test_versions! {
        fn not_available_raise_na_error(api, version) if "< v2.7" {
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
              extend schema
                @link(
                  url: "https://specs.apollo.dev/federation/${version}"
                  import: ["@key", "@override"]
                )

              type Query {
                foo: User
              }

              type User @key(fields: "id") {
                id: ID
                name: String @override(from: "bar", label: "public")
              }
            "#, version)),
                service("bar", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type Query {
                bar: User
              }

              type User @key(fields: "id") {
                id: ID
                name: String
              }
            "#, version)),
            ]);

            assert_composition_failure(&result);

            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[foo] Unknown argument "label" on directive "@override"."#)
                    .code("INVALID_GRAPHQL")],
            );
        }

        fn available_allow_to_use(api, version) if ">= v2.7" {
            // User is available in all subgraphs
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override"])

            type Query {
              foo: User
            }

            type User @key(fields: "id") {
              id: ID
              name: String @override(from: "bar", label: "public")
            }
          "#, version)),
                service("bar", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

              type Query {
                bar: User
              }

              type User @key(fields: "id") {
                id: ID
                name: String
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: BAR, key: "id")
          @join__type(graph: FOO, key: "id") {
          id: ID
          name: String
            @join__field(graph: BAR, overrideLabel: "public")
            @join__field(graph: FOO, override: "bar", overrideLabel: "public")
        }
      "#);

            // User is NOT available in all subgraphs
            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override", "@shareable"])

            type Query {
              foo: User
            }

            type User @key(fields: "id") {
              id: ID
              name: String @override(from: "bar", label: "public") @shareable
            }
          "#, version)),
                service("bar", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

              type Query {
                bar: User
              }

              type User @key(fields: "id") {
                id: ID
                name: String @shareable
              }
            "#, version)),
                service("baz", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

              type User @key(fields: "id") {
                id: ID
                name: String @shareable
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: BAR, key: "id")
          @join__type(graph: BAZ, key: "id")
          @join__type(graph: FOO, key: "id") {
          id: ID
          name: String
            @join__field(graph: BAZ)
            @join__field(graph: BAR, overrideLabel: "public")
            @join__field(graph: FOO, override: "bar", overrideLabel: "public")
        }
      "#);

            let result = api.compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override", "@shareable"])

            type Query {
              foo: User
            }

            type User @key(fields: "id") {
              id: ID
              name: String @override(from: "bar", label: "public") @shareable
            }
          "#, version)),
                service("bar", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

              type Query {
                bar: User
              }

              type User @key(fields: "id") {
                id: ID
                name: String @shareable
              }
            "#, version)),
                service("baz", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

              type User @key(fields: "id") {
                id: ID
              }
            "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: BAR, key: "id")
          @join__type(graph: BAZ, key: "id")
          @join__type(graph: FOO, key: "id") {
          id: ID
          name: String
            @join__field(graph: BAR, overrideLabel: "public")
            @join__field(graph: FOO, override: "bar", overrideLabel: "public")
        }
      "#);
        }

        fn label_percent_11_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(-11)");
        }

        fn label_percent_1_1_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(-1.1)");
        }

        fn label_percent_1_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(-1)");
        }

        fn label_percent_0_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(-0)");
        }

        fn label_percent_0_should_fail_2(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(+0)");
        }

        fn label_percent_0_00_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(+0.00)");
        }

        fn label_percent_0_00_should_fail_2(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(-0.00)");
        }

        fn label_percent_1_123456789_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(1.123456789)");
        }

        fn label_percent_101_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "percent(101)");
        }

        fn label_0foo_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "0foo");
        }

        fn label_foo_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, "_foo");
        }

        fn label_foo_should_fail_2(api, version) if ">= v2.7" {
            label_should_fail(api, version, "-foo");
        }

        fn label_foo_should_fail_3(api, version) if ">= v2.7" {
            label_should_fail(api, version, ":foo");
        }

        fn label_foo_should_fail_4(api, version) if ">= v2.7" {
            label_should_fail(api, version, ".foo");
        }

        fn label_foo_should_fail_5(api, version) if ">= v2.7" {
            label_should_fail(api, version, "/foo");
        }

        fn label_foo_should_fail_6(api, version) if ">= v2.7" {
            label_should_fail(api, version, "!foo");
        }

        fn label_foo_should_fail_7(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo%");
        }

        fn label_foo_should_fail_8(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo(");
        }

        fn label_foo_should_fail_9(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo)");
        }

        fn label_foo_should_fail_10(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo[");
        }

        fn label_foo_should_fail_11(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo]");
        }

        fn label_foo_should_fail_12(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo{");
        }

        fn label_foo_should_fail_13(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo}");
        }

        fn label_foo_should_fail_14(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo<");
        }

        fn label_foo_should_fail_15(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo>");
        }

        fn label_foo_should_fail_16(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo=");
        }

        fn label_foo_should_fail_17(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo?");
        }

        fn label_foo_should_fail_18(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo!");
        }

        fn label_foo_should_fail_19(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo@");
        }

        fn label_foo_should_fail_20(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo#");
        }

        fn label_foo_should_fail_21(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo$");
        }

        fn label_foo_should_fail_22(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo^");
        }

        fn label_foo_should_fail_23(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo&");
        }

        fn label_foo_should_fail_24(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo*");
        }

        fn label_foo_should_fail_25(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo~");
        }

        fn label_foo_should_fail_26(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo`");
        }

        fn label_foo_should_fail_27(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo|");
        }

        fn label_foo_should_fail_28(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo;");
        }

        fn label_foo_should_fail_29(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo,");
        }

        fn label_foo_should_fail_30(api, version) if ">= v2.7" {
            label_should_fail(api, version, "foo ");
        }

        fn label_foo_should_fail_31(api, version) if ">= v2.7" {
            label_should_fail(api, version, " foo");
        }

        fn label_0_should_fail(api, version) if ">= v2.7" {
            label_should_fail(api, version, " 0");
        }


        fn label_percent_0_00_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "percent(0.00)");
        }

        fn label_percent_1_00_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "percent(1.00)");
        }

        fn label_percent_1_12345678_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "percent(1.12345678)");
        }

        fn label_percent_100_00_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "percent(100.00)");
        }

        fn label_foo_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "foo");
        }

        fn label_foo_bar_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "foo_bar");
        }

        fn label_percent1_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "percent1");
        }

        fn label_f0_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "f0");
        }

        fn label_f_should_pass(api, version) if ">= v2.7" {
            label_should_pass(api, version, "f_");
        }

        fn label_f_should_pass_2(api, version) if ">= v2.7" {
            label_should_pass(api, version, "f-");
        }

        fn label_f_should_pass_3(api, version) if ">= v2.7" {
            label_should_pass(api, version, "f:");
        }

        fn label_f_should_pass_4(api, version) if ">= v2.7" {
            label_should_pass(api, version, "f.");
        }

        fn label_f_should_pass_5(api, version) if ">= v2.7" {
            label_should_pass(api, version, "f/");
        }

        fn should_detect_unsatisfiable_paths(api, version) if ">= v2.7" {
            let result = api.compose_services(&[
                service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override"])

          type Query {
            user(id: ID!): User
          }

          type User @key(fields: "id") {
            id: ID!
            group: Group @override(from: "b") # no label
          }

          type Group @key(fields: "id") {
            id: ID!
            level: Int
          }
        "#, version)),
                service("b", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@requires", "@external", "@override"])

          type User @key(fields: "id") {
            id: ID!
            group: Group!
            groupLevel: ID! @requires(fields: "group { level }")
          }

          type Group @key(fields: "id") {
            id: ID
            level: Int @external
          }
        "#, version)),
            ]);

            assert_composition_success(&result);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type User
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID!
          group: Group
            @join__field(graph: A, override: "b", type: "Group")
            @join__field(graph: B, type: "Group!", usedOverridden: true)
          groupLevel: ID! @join__field(graph: B, requires: "group { level }")
        }
      "#);

            to_contain_graphql(result.supergraph_sdl(), r#"
        type Group
          @join__type(graph: A, key: "id")
          @join__type(graph: B, key: "id") {
          id: ID
            @join__field(graph: A, type: "ID!")
            @join__field(graph: B, type: "ID")
          level: Int
            @join__field(graph: A)
            @join__field(graph: B, external: true)
        }
      "#);

            let result = api.compose_services(&[
                service("a", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@override"])

          type Query {
            user(id: ID!): User
          }

          type User @key(fields: "id") {
            id: ID!
            group: Group @override(from: "b", label: "public")
          }

          type Group @key(fields: "id") {
            id: ID!
            level: Int
          }
        "#, version)),
                service("b", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@requires", "@external", "@override"])

          type User @key(fields: "id") {
            id: ID!
            group: Group!
            groupLevel: ID! @requires(fields: "group { level }")
          }

          type Group @key(fields: "id") {
            id: ID
            level: Int @external
          }
        "#, version)),
            ]);

            assert_composition_failure(&result);
        }
    }
}
