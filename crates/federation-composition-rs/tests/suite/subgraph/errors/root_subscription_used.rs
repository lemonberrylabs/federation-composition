//! Port of `__tests__/subgraph/errors/ROOT_SUBSCRIPTION_USED.spec.ts`.

use crate::shared::*;

test_versions! {
    fn root_subscription_used(api, version) {
        assert_errors_contain(
            &api.compose_services(&[service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

            type Subscription {
              users: [User!]!
            }

            schema {
              subscription: RootSubscription
            }

            type RootSubscription {
              users: [User!]!
            }

            type User @key(fields: "id") {
              id: ID
              friends: [User!]!
            }
          "#, version))]),
            &[ErrorMatcher::new()
                .message_containing(
                    r#"The schema has a type named "Subscription" but it is not set as the subscription root type ("RootSubscription" is instead): this is not supported by federation. If a root type does not use its default name, there should be no other type with that default name."#,
                )
                .code("ROOT_SUBSCRIPTION_USED")],
        );
    }
}
