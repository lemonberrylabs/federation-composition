//! Port of `__tests__/supergraph/base.spec.ts`.

use crate::shared::*;

/// `expect(result.supergraphSdl).toContain(substring)`; `undefined` fails, as in vitest.
#[track_caller]
fn assert_supergraph_contains(result: &CompositionResult, substring: &str) {
    let sdl = result.supergraph_sdl().expect("supergraphSdl is undefined");
    assert!(
        sdl.contains(substring),
        "expected supergraphSdl to contain:\n{substring}\nsupergraphSdl:\n{sdl}"
    );
}

/// `expect(result.supergraphSdl).not.toContain(substring)`; `undefined` fails, as in vitest.
#[track_caller]
fn assert_supergraph_not_contains(result: &CompositionResult, substring: &str) {
    let sdl = result.supergraph_sdl().expect("supergraphSdl is undefined");
    assert!(
        !sdl.contains(substring),
        "expected supergraphSdl not to contain:\n{substring}\nsupergraphSdl:\n{sdl}"
    );
}

test_versions! {
    fn detect_name_collisions(api, version) {
        // "detected exact same names"
        assert_throws(
            api.try_compose_services(&[
                service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "name") {
              name: String
            }

            type Query {
              user: User
            }
          "#, version)),
                service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Member @key(fields: "name") {
              name: String
            }

            type Query {
              member: Member
            }
          "#, version)),
            ]),
            Some("A subgraph named foo already exists"),
        );

        // "names are almost identical (one starts with a capital letter)"
        assert_does_not_throw(api.try_compose_services(&[
            service("Foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "name") {
              name: String
            }

            type Query {
              user: User
            }
          "#, version)),
            service("foo", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Member @key(fields: "name") {
              name: String
            }

            type Query {
              member: Member
            }
          "#, version)),
        ]));
    }

    fn suffix_identical_foo_vs_foo_names_with_a_number(api, version) {
        let result = api.compose_services(&[
            service("Foo", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type User @key(fields: "name") {
            name: String
          }

          type Query {
            user: User
          }
        "#, version)),
            service("foo", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type Member @key(fields: "name") {
            name: String
          }

          type Query {
            member: Member
          }
        "#, version)),
        ]);

        assert_composition_success(&result);

        to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
      enum join__Graph {
        FOO_1 @join__graph(name: "Foo", url: "")
        FOO_2 @join__graph(name: "foo", url: "")
      }
    "#));
    }

    fn not_detect_url_collisions(api, version) {
        // "detected exact same urls"
        assert_does_not_throw(api.try_compose_services(&[
            service_with_url("foo", "http://foo.com", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type User @key(fields: "name") {
              name: String
            }

            type Query {
              user: User
            }
          "#, version)),
            service_with_url("bar", "http://foo.com", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Member @key(fields: "name") {
              name: String
            }

            type Query {
              member: Member
            }
          "#, version)),
        ]));
    }

    fn executable_directive_definition_is_retained_in_supergraph(api, version) {
        let result = api.compose_services(&[service_with_url("foo", "http://foo.com", graphql!(r#"
        extend schema
          @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

        directive @a(n: Int) on FIELD

        type Query {
          a: Int
        }
      "#, version))]);
        assert_supergraph_contains(&result, "directive @a(n: Int)");
    }

    fn executable_directive_definition_is_retained_if_defined_identical_in_all_subgraphs(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            a: Int
          }
      "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            b: Int
          }
        "#, version)),
        ]);
        assert_supergraph_contains(&result, "directive @a(n: Int)");
    }

    fn executable_directive_definition_is_omitted_if_only_defined_within_a_single_subgraph(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
        extend schema
          @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type Query {
            b: Int
          }
        "#, version)),
        ]);
        assert_supergraph_not_contains(&result, "directive @a(n: Int)");
    }

    fn executable_directive_only_contains_locations_shared_between_all_subgraphs(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD | FRAGMENT_SPREAD

          type Query {
            b: Int
          }
          "#, version)),
        ]);
        assert!(result.supergraph_sdl().is_some());
        to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
      directive @a(n: Int) on FIELD
    "#));
        not_to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
      directive @a(n: Int) on FIELD | FRAGMENT_SPREAD
    "#));
    }

    fn executable_directive_is_removed_if_no_locations_are_shared_between_all_subgraphs(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FRAGMENT_SPREAD

          type Query {
            b: Int
          }
          "#, version)),
        ]);

        assert!(result.supergraph_sdl().is_some());
        assert_supergraph_not_contains(&result, "directive @a(n: Int)");
    }

    fn executable_directive_is_only_included_with_executable_definition_locations(api, version) {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD | FIELD_DEFINITION

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          directive @a(n: Int) on FIELD | FIELD_DEFINITION

          type Query {
            b: Int
          }
          "#, version)),
        ]);

        assert!(result.supergraph_sdl().is_some());
        to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
      directive @a(n: Int) on FIELD
    "#));
    }

    // if (version !== "v2.0")
    fn executable_directive_that_is_also_a_compose_directive_is_included_with_executable_definition_and_schema_definition_locations(api, version) if "> v2.0" {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@composeDirective"])
            @link(url: "https://a.dev/a/v1.0", import: ["@a"])
            @composeDirective(name: "@a")

          directive @a(n: Int) on FIELD | FIELD_DEFINITION

          type Query {
            a: Int
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: [ "@composeDirective"])
            @link(url: "https://a.dev/a/v1.0", import: ["@a"])
            @composeDirective(name: "@a")

          directive @a(n: Int) on FIELD | FIELD_DEFINITION

          type Query {
            b: Int
          }
          "#, version)),
        ]);

        assert!(result.supergraph_sdl().is_some());
        to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        directive @a(n: Int) on FIELD | FIELD_DEFINITION
      "#));
    }

    // if (version !== "v2.0")
    fn executable_directive_that_is_also_a_compose_directive_and_used_within_the_subgraph_schemas_is_only_included_with_executable_definition_locations_and_schema_definition_locations(api, version) if "> v2.0" {
        let result = api.compose_services(&[
            service_with_url("a", "http://a.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@composeDirective"])
            @link(url: "https://a.dev/a/v1.0", import: ["@a"])
            @composeDirective(name: "@a")

          directive @a(n: Int) on FIELD | FIELD_DEFINITION

          type Query {
            a: Int @a
          }
        "#, version)),
            service_with_url("b", "http://b.com", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: [ "@composeDirective"])
            @link(url: "https://a.dev/a/v1.0", import: ["@a"])
            @composeDirective(name: "@a")

          directive @a(n: Int) on FIELD | FIELD_DEFINITION

          type Query {
            b: Int @a
          }
          "#, version)),
        ]);
        assert!(result.supergraph_sdl().is_some());
        to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        directive @a(n: Int) on FIELD | FIELD_DEFINITION
      "#));
        to_contain_graphql(result.supergraph_sdl(), graphql!(r#"
        type Query @join__type(graph: A) @join__type(graph: B) {
          a: Int @a @join__field(graph: A)
          b: Int @a @join__field(graph: B)
        }
      "#));
    }
}
