//! Port of `__tests__/subgraph/errors/ROOT_MUTATION_USED.spec.ts`.

use crate::shared::*;

test_versions! {
    fn root_mutation_used(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Mutation {
              users: [User!]!
            }

            schema {
              mutation: RootMutation
            }

            type RootMutation {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
              friends: [User!]!
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"The schema has a type named "Mutation" but it is not set as the mutation root type ("RootMutation" is instead): this is not supported by federation. If a root type does not use its default name, there should be no other type with that default name."#,
                )
                .code("ROOT_MUTATION_USED")],
        );
    }
}
