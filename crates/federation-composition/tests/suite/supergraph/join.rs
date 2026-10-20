//! Port of `__tests__/supergraph/join.spec.ts`.

use crate::shared::*;

mod join_spec_version {
    use super::*;

    /// One row of `test.each(Object.entries(federationVersionToJoinSpecVersion))("Federation %s == Join %s", ..)`.
    fn federation_to_join(api: Api, fed_version: &str, join_version: &str) {
        // The row comes from `federationVersionToJoinSpecVersion`.
        assert_eq!(federation_version_to_join_spec_version(fed_version), join_version);

        let result = api.compose_services(&[service(
            "a",
            graphql!(
                r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${fed_version}" import: ["@key"])

            type User @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              users: [User]
            }
          "#,
                fed_version
            ),
        )]);

        assert_composition_success(&result);

        let expected = tpl!(
            r#"@link(url: "https://specs.apollo.dev/join/${join_version}", for: EXECUTION)"#,
            join_version
        );
        let sdl = result.supergraph_sdl().expect("supergraphSdl is undefined");
        assert!(
            sdl.contains(&expected),
            "expected supergraphSdl to match:\n{expected}\nsupergraphSdl:\n{sdl}"
        );
    }

    test_implementations! {
        fn federation_v2_0_join_v0_3(api) { federation_to_join(api, "v2.0", "v0.3"); }
        fn federation_v2_1_join_v0_3(api) { federation_to_join(api, "v2.1", "v0.3"); }
        fn federation_v2_2_join_v0_3(api) { federation_to_join(api, "v2.2", "v0.3"); }
        fn federation_v2_3_join_v0_3(api) { federation_to_join(api, "v2.3", "v0.3"); }
        fn federation_v2_4_join_v0_3(api) { federation_to_join(api, "v2.4", "v0.3"); }
        fn federation_v2_5_join_v0_3(api) { federation_to_join(api, "v2.5", "v0.3"); }
        fn federation_v2_6_join_v0_3(api) { federation_to_join(api, "v2.6", "v0.3"); }
        fn federation_v2_7_join_v0_4(api) { federation_to_join(api, "v2.7", "v0.4"); }
        fn federation_v2_8_join_v0_5(api) { federation_to_join(api, "v2.8", "v0.5"); }
        fn federation_v2_9_join_v0_5(api) { federation_to_join(api, "v2.9", "v0.5"); }

        fn federation_v1_0_join_v0_3(api) {
            let result = api.compose_services(&[service("a", graphql!(r#"
            type User @key(fields: "id") {
              id: ID!
              name: String!
            }

            type Query {
              users: [User]
            }
          "#))]);

            assert_composition_success(&result);

            let expected = r#"@link(url: "https://specs.apollo.dev/join/v0.3", for: EXECUTION)"#;
            let sdl = result.supergraph_sdl().expect("supergraphSdl is undefined");
            assert!(sdl.contains(expected), "expected supergraphSdl to match:\n{expected}\nsupergraphSdl:\n{sdl}");
        }
    }
}
