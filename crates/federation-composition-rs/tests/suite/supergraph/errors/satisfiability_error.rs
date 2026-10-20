//! Port of `__tests__/supergraph/errors/SATISFIABILITY_ERROR.spec.ts`.

use crate::shared::*;

/// Whether the JS `RegExp` built from `pattern` (`new RegExp(pattern)`, no flags) matches
/// somewhere in `text`, for a `pattern` whose only regex syntax is `.`: it matches any character
/// except a line terminator, and every other character matches itself.
fn matches_js_regex_with_dots(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    assert!(
        !pattern
            .iter()
            .any(|c| matches!(c, '\\' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '|')),
        "the pattern has regex syntax other than `.`"
    );
    assert!(
        !pattern.windows(2).any(|w| w[0] == '{' && w[1].is_ascii_digit()),
        "the pattern has a quantifier"
    );
    let is_line_terminator = |c: char| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}');
    (0..=text.len().saturating_sub(pattern.len())).any(|start| {
        text.len() >= pattern.len()
            && pattern
                .iter()
                .zip(&text[start..])
                .all(|(&p, &c)| if p == '.' { !is_line_terminator(c) } else { p == c })
    })
}

test_versions! {
    fn cannot_satisfy_require_conditions(api, version) {
        let result = api.compose_services(&[
          service("users", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

          type Query {
            users: [User]
          }

          type User @key(fields: "id name") {
            id: ID!
            name: String!
          }
        "#, version)),
          service("feed", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external", "@requires"])

          type Query {
            usersByIds(ids: [ID!]!): [User!]!
          }

          extend type User @key(fields: "id") {
            id: ID @external
            name: String! @external
            comments: [String] @requires(fields: "name")
          }
        "#, version)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
            The following supergraph API query:
            {
              usersByIds(ids: []) {
                comments
              }
            }
            cannot be satisfied by the subgraphs because:
            - from subgraph "feed":
              - cannot satisfy @require conditions on field "User.comments".
              - cannot move to subgraph "users" using @key(fields: "id name") of "User", the key field(s) cannot be resolved from subgraph "feed".
        "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
            The following supergraph API query:
            {
              usersByIds(ids: []) {
                name
              }
            }
            cannot be satisfied by the subgraphs because:
            - from subgraph "feed":
              - field "User.name" is not resolvable because marked @external.
              - cannot move to subgraph "users" using @key(fields: "id name") of "User", the key field(s) cannot be resolved from subgraph "feed".
          "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
          The following supergraph API query:
          {
            users {
              comments
            }
          }
          cannot be satisfied by the subgraphs because:
          - from subgraph "users": cannot find field "User.comments".
          - from subgraph "feed": cannot satisfy @require conditions on field "User.comments".
        "#)).code("SATISFIABILITY_ERROR")]);
    }

    fn insufficient_key_fields_to_move_between_graphs(api, version) {
        // Cannot resolve User.name from Query.randomUser because key field "id" from subgraph "feed"
        // is not enough to fullfil the @key(fields: "id name") of User from subgraph "users"
        assert_errors_equal(
          api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Query {
              userById: User
            }

            type User @key(fields: "id name") {
              id: ID!
              name: String!
            }
          "#, version)),
            service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            type Query {
              randomUser: User
            }

            extend type User @key(fields: "id") {
              id: ID @external
              comments: [String]
            }
          "#, version)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  randomUser {\n",
                    "    name\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"feed\":\n",
                    "  - cannot find field \"User.name\".\n",
                    "  - cannot move to subgraph \"users\" using @key(fields: \"id name\") of \"User\", the key field(s) cannot be resolved from subgraph \"feed\".",
                  )).code("SATISFIABILITY_ERROR"),
          ],
        );

        // Both User.{name,email,profile} cannot be resolved as Federation Gateway cannot move between subgraphs using identical @key.
        // The key fields are not the same ("id" is not enough to move from subgraph "a" to subgraph "b" that requires "id email").
        assert_errors_equal(
          api.compose_services(&[
            service("a", graphql!(r#"
            extend type User @key(fields: "id") {
              id: ID @external
              tags: [String]
            }

            type Query {
              usersById(ids: [ID!]!): [User!]!
            }
          "#)),
            service("b", graphql!(r#"
            type User @key(fields: "id email") {
              id: ID
              name: String
              email: String
              profile: String
            }

            type Query {
              users(ids: [ID!]!): [User!]!
            }
          "#)),
          ]),
          &[
              // sorted by the guild position (`.sort((a, b) => a[1] - b[1])`): email (1), name (2), profile (3)
                ErrorMatcher::new().message_containing(concat!(
                      "The following supergraph API query:\n",
                      "{\n",
                      "  usersById(ids: []) {\n",
                      "    email\n",
                      "  }\n",
                      "}\n",
                      "cannot be satisfied by the subgraphs because:\n",
                      "- from subgraph \"a\":\n",
                      "  - cannot find field \"User.email\".\n",
                      "  - cannot move to subgraph \"b\" using @key(fields: \"id email\") of \"User\", the key field(s) cannot be resolved from subgraph \"a\".",
                    )).code("SATISFIABILITY_ERROR"),
                ErrorMatcher::new().message_containing(concat!(
                      "The following supergraph API query:\n",
                      "{\n",
                      "  usersById(ids: []) {\n",
                      "    name\n",
                      "  }\n",
                      "}\n",
                      "cannot be satisfied by the subgraphs because:\n",
                      "- from subgraph \"a\":\n",
                      "  - cannot find field \"User.name\".\n",
                      "  - cannot move to subgraph \"b\" using @key(fields: \"id email\") of \"User\", the key field(s) cannot be resolved from subgraph \"a\".",
                    )).code("SATISFIABILITY_ERROR"),
                ErrorMatcher::new().message_containing(concat!(
                      "The following supergraph API query:\n",
                      "{\n",
                      "  usersById(ids: []) {\n",
                      "    profile\n",
                      "  }\n",
                      "}\n",
                      "cannot be satisfied by the subgraphs because:\n",
                      "- from subgraph \"a\":\n",
                      "  - cannot find field \"User.profile\".\n",
                      "  - cannot move to subgraph \"b\" using @key(fields: \"id email\") of \"User\", the key field(s) cannot be resolved from subgraph \"a\".",
                    )).code("SATISFIABILITY_ERROR"),
          ],
        );
    }

    fn insufficient_key_fields_to_move_between_graphs_entity_many_level_deep_from_root_type(api, version) {
        // Same as above, but with an entity deep down the path.
        // Cannot resolve User.name from Query.randomUser because key field "id" from subgraph "feed"
        // is not enough to fullfil the @key(fields: "id name") of User from subgraph "users".
        let result = api.compose_services(&[
          service("users", graphql!(r#"
        extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

        type Query {
          userById(id: ID): User
        }

        type User @key(fields: "id name") {
          id: ID!
          name: String!
        }
      "#, version)),
          service("feed", graphql!(r#"
        extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

        type Query {
          users: UserConnection!
          randomUser: UserEdge
        }

        type UserConnection {
          edges: [UserEdge]
        }

        type UserEdge {
          node: User
        }

        extend type User @key(fields: "id") {
          id: ID @external
          comments: [String]
        }
      "#, version)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                users {
                  edges {
                    node {
                      name
                    }
                  }
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "feed":
                - cannot find field "User.name".
                - cannot move to subgraph "users" using @key(fields: "id name") of "User", the key field(s) cannot be resolved from subgraph "feed".
          "#)).code("SATISFIABILITY_ERROR")]);
    }

    // ADD IT TO THE COLLECTION
    fn works_with_override(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("foo", graphql!(r#"
            directive @override(from: String!) on FIELD_DEFINITION

            type Query {
              view: Queries!
            }

            type Queries {
              aaa: [String!]! @override(from: "bar")
              bbb: String! @override(from: "bar")
              ccc: String! @override(from: "bar")
              user: User! @override(from: "bar")
            }

            type User {
              id: ID! @override(from: "bar")
              name: String! @override(from: "bar")
            }
          "#)),
            service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@shareable", "@inaccessible"])

            type Query @shareable {
              _internal: Boolean @inaccessible
              view: Queries!
            }

            type Queries @shareable {
              aaa: [String!]!
              bbb: String!
              ccc: String!
              ddd: String!
              eee: String!
              fff: String!
              user: User!
            }

            type User {
              id: ID!
              name: String!
            }
          "#, version)),
            service("baz", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@inaccessible", "@shareable"])

            type Query {
              _bazInternal: Boolean @inaccessible
            }

            type Queries {
              _bazQueriesInternal: Boolean @inaccessible
            }
          "#, version)),
            service("qux", graphql!(r#"
            type Query {
              qux: String
            }
          "#)),
            service("quux", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@inaccessible", "@shareable"])

            type Query {
              _quux: Boolean @inaccessible
              view: Queries! @shareable
            }

            type Queries {
              _quux: Boolean @inaccessible
              quux: String @shareable
            }
          "#, version)),
          ]),
        );

        // {
        //   view {
        //     aaa
        //   }
        // }
        // cannot be satisfied by the subgraphs because:
        // - from subgraph "baz":
        //   - cannot find field "Queries.aaa".
        //   - cannot move to subgraph "bar", which has field "Queries.aaa", because type "Queries" has no @key defined in subgraph "bar".
        // - from subgraph "quux":
        //   - cannot find field "Queries.aaa".
        //   - cannot move to subgraph "bar", which has field "Queries.aaa", because type "Queries" has no @key defined in subgraph "bar".
        //
        // {
        //   view {
        //     bbb
        //   }
        // }
        // cannot be satisfied by the subgraphs because:
        // - from subgraph "baz":
        //   - cannot find field "Queries.bbb".
        //   - cannot move to subgraph "bar", which has field "Queries.bbb", because type "Queries" has no @key defined in subgraph "bar".
        // - from subgraph "quux":
        //   - cannot find field "Queries.bbb".
        //   - cannot move to subgraph "bar", which has field "Queries.bbb", because type "Queries" has no @key defined in subgraph "bar".
        //
        // {
        //   view {
        //     ccc
        //   }
        // }
        // cannot be satisfied by the subgraphs because:
        // - from subgraph "baz":
        //   - cannot find field "Queries.ccc".
        //   - cannot move to subgraph "bar", which has field "Queries.ccc", because type "Queries" has no @key defined in subgraph "bar".
        // - from subgraph "quux":
        //   - cannot find field "Queries.ccc".
        //   - cannot move to subgraph "bar", which has field "Queries.ccc", because type "Queries" has no @key defined in subgraph "bar".
        //
        // {
        //   view {
        //     user {
        //       ...
        //     }
        //   }
        // }
        // cannot be satisfied by the subgraphs because:
        // - from subgraph "baz":
        //   - cannot find field "Queries.user".
        //   - cannot move to subgraph "bar", which has field "Queries.user", because type "Queries" has no @key defined in subgraph "bar".
        // - from subgraph "quux":
        //   - cannot find field "Queries.user".
        //   - cannot move to subgraph "bar", which has field "Queries.user", because type "Queries" has no @key defined in subgraph "bar".

        assert_errors_contain(
          api.compose_services(&[
            service("foo", graphql!(r#"
            directive @override(from: String!) on FIELD_DEFINITION

            type Query {
              view: Queries!
            }

            type Queries {
              aaa: [String!]! @override(from: "bar")
              bbb: String! @override(from: "bar")
              ccc: String! @override(from: "bar")
              user: User! @override(from: "bar")
            }

            type User {
              id: ID! @override(from: "bar")
              name: String! @override(from: "bar")
            }
          "#)),
            service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@shareable", "@inaccessible"])

            type Query @shareable {
              _internal: Boolean @inaccessible
              view: Queries!
            }

            type Queries @shareable {
              aaa: [String!]!
              bbb: String!
              ccc: String!
              ddd: String!
              eee: String!
              fff: String!
              user: User!
            }

            type User {
              id: ID!
              name: String!
            }
          "#, version)),
            service("baz", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@inaccessible", "@shareable"])

            type Query {
              _bazInternal: Boolean @inaccessible
            }

            type Queries {
              _bazQueriesInternal: Boolean @inaccessible
            }
          "#, version)),
            service("qux", graphql!(r#"
            type Query {
              qux: String
            }
          "#)),
            service("quux", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@inaccessible", "@shareable"])

            type Query {
              _quux: Boolean @inaccessible
              view: Queries!
            }

            type Queries {
              _quux: Boolean @inaccessible
              quux: String @shareable
            }
          "#, version)),
          ]),
          &[
              ErrorMatcher::new().message_containing(r#"Non-shareable field "Query.view" is resolved from multiple subgraphs: it is resolved from subgraphs "bar", "foo" and "quux" and defined as non-shareable in subgraph "quux""#).code("INVALID_FIELD_SHARING"),
          ],
        );
    }

    fn insufficient_key_fields_to_move_between_graphs_completely_different_key_fields(api, version) {
        assert_errors_equal(
          api.compose_services(&[
            service("users", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Query {
              users: [User]
            }

            type User @key(fields: "id") {
              id: ID!
              name: String!
            }
          "#, version)),
            service("feed", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Query {
              usersByIds(ids: [ID!]!): [User!]!
            }

            extend type User @key(fields: "email") {
              email: String
              comments: [String]
            }
          "#, version)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  users {\n",
                    "    email\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"users\":\n",
                    "  - cannot find field \"User.email\".\n",
                    "  - cannot move to subgraph \"feed\" using @key(fields: \"email\") of \"User\", the key field(s) cannot be resolved from subgraph \"users\".",
                  )).code("SATISFIABILITY_ERROR"),
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  users {\n",
                    "    comments\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"users\":\n",
                    "  - cannot find field \"User.comments\".\n",
                    "  - cannot move to subgraph \"feed\" using @key(fields: \"email\") of \"User\", the key field(s) cannot be resolved from subgraph \"users\".",
                  )).code("SATISFIABILITY_ERROR"),
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  usersByIds(ids: []) {\n",
                    "    id\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"feed\":\n",
                    "  - cannot find field \"User.id\".\n",
                    "  - cannot move to subgraph \"users\" using @key(fields: \"id\") of \"User\", the key field(s) cannot be resolved from subgraph \"feed\".",
                  )).code("SATISFIABILITY_ERROR"),
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  usersByIds(ids: []) {\n",
                    "    name\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"feed\":\n",
                    "  - cannot find field \"User.name\".\n",
                    "  - cannot move to subgraph \"users\" using @key(fields: \"id\") of \"User\", the key field(s) cannot be resolved from subgraph \"feed\".",
                  )).code("SATISFIABILITY_ERROR"),
          ],
        );
    }

    fn cannot_move_as_not_shareable_and_no_keys(api, version) {
        // Cannot resolve User.name from Query.usersByAge as User has no @key to move to subgraph "a"
        // Cannot resolve User.age from Query.users as User has no @key to move to subgraph "b"
        assert_errors_equal(
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type User {
              name: String
            }

            type Query {
              users(filter: UsersFilter, role: Role): [User]
            }

            input UsersFilter {
              limit: Int!
              after: ID!
              user: UserFilter
            }

            input UserFilter {
              role: Role
            }

            enum Role {
              ADMIN
              USER
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            extend type User {
              age: Int
            }

            type Query {
              """
              Note: age is nullable, but limit is not
              """
              usersByAge(age: Int, limit: Int!): [User]
            }
          "#, version)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  usersByAge(age: 0, limit: 0) {\n",
                    "    name\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"b\":\n",
                    "  - cannot find field \"User.name\".\n",
                    "  - cannot move to subgraph \"a\", which has field \"User.name\", because type \"User\" has no @key defined in subgraph \"a\".",
                  )).code("SATISFIABILITY_ERROR"),
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  users(filter: { limit: 0, after: \"<any id>\" }, role: ADMIN) {\n",
                    "    age\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"a\":\n",
                    "  - cannot find field \"User.age\".\n",
                    "  - cannot move to subgraph \"b\", which has field \"User.age\", because type \"User\" has no @key defined in subgraph \"b\".",
                  )).code("SATISFIABILITY_ERROR"),
          ],
        );

        // Cannot resolve User.name from Query.usersByAge as User has no @key to move to subgraph "a"
        assert_errors_equal(
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type User {
              name: String
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            extend type User {
              age: Int
            }

            type Query {
              usersByAge(age: Int): [User]
            }
          "#, version)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  usersByAge(age: 0) {\n",
                    "    name\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"b\":\n",
                    "  - cannot find field \"User.name\".\n",
                    "  - cannot move to subgraph \"a\", which has field \"User.name\", because type \"User\" has no @key defined in subgraph \"a\".",
                  )).code("SATISFIABILITY_ERROR"),
          ],
        );
    }

    fn cannot_move_as_not_shareable_and_no_keys_mutation_and_subscription(api, version) {
        // Cannot resolve User.name from Query.usersByAge as User has no @key to move to subgraph "a"
        // Cannot resolve User.age from Query.users as User has no @key to move to subgraph "b"
        let result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

          type User {
            name: String
          }

          type Query {
            users: [User]
          }
        "#, version)),
          service("b", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

          extend type User {
            age: Int
          }

          type Mutation {
            createRandomUser: User
          }

          type Subscription {
            onUserCreated: User
          }
        "#, version)),
        ]);

        assert_composition_failure(&result);

        // `if (api.library === "guild") { .. }`
        // Why??????
        // Because Apollo does not throw an error for mutation if subscription is detected, LOL.
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(concat!(
                "The following supergraph API query:\n",
                "mutation {\n",
                "  createRandomUser {\n",
                "    name\n",
                "  }\n",
                "}\n",
                "cannot be satisfied by the subgraphs because:\n",
                "- from subgraph \"b\":\n",
                "  - cannot find field \"User.name\".\n",
                "  - cannot move to subgraph \"a\", which has field \"User.name\", because type \"User\" has no @key defined in subgraph \"a\".",
              )).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
            The following supergraph API query:
            mutation {
              createRandomUser {
                name
              }
            }
            cannot be satisfied by the subgraphs because:
            - from subgraph "b":
              - cannot find field "User.name".
              - cannot move to subgraph "a", which has field "User.name", because type "User" has no @key defined in subgraph "a".
          "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(concat!(
                "The following supergraph API query:\n",
                "{\n",
                "  users {\n",
                "    age\n",
                "  }\n",
                "}\n",
                "cannot be satisfied by the subgraphs because:\n",
                "- from subgraph \"a\":\n",
                "  - cannot find field \"User.age\".\n",
                "  - cannot move to subgraph \"b\", which has field \"User.age\", because type \"User\" has no @key defined in subgraph \"b\".",
              )).code("SATISFIABILITY_ERROR")]);
    }

    fn unreachable_from_root(api, version) {
        // When User is not queryable (not referenced by a Query type and its dependencies, at any level)
        // it can be ignored by the SATISFIABILITY_ERROR check.
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type User {
              name: String
            }

            type Query {
              users: [String]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            extend type User {
              age: Int
            }

            type Query {
              usersByAge(age: Int): [String]
            }
          "#, version)),
          ]),
        );
    }

    fn identical_keys_fields(api, version) {
        // User.{profile,email} can be resolved as Federation Gateway can move between subgraphs using identical @key
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            extend type User @key(fields: "id") {
              id: ID @external
              email: String
            }

            type Query {
              usersById(ids: [ID!]!): [User!]!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type User @key(fields: "id") {
              id: ID
              profile: String
            }

            type Query {
              users(ids: [ID!]!): [User!]!
            }
          "#, version)),
          ]),
        );
    }

    fn unresolvable_because_of_external_no_idea(api, version) {
        let mut result = api.compose_services(&[
          service("foo", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@inaccessible"])

            type Product @key(fields: "publicId fooCriteria") @key(fields: "key { code type } fooCriteria") {
              fooId: String! @inaccessible
              fooCriteria: String @inaccessible
              publicId: ID!
              key: Key
            }

            type Key {
              code: Int!
              type: ProductType!
            }

            enum ProductType {
              PUBLIC
              PRIVATE
            }

            type Query {
              productInFoo: Product
            }
        "#, version)),
          service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            type Product @key(fields: "publicId fooCriteria") {
              publicId: ID! @external
              fooCriteria: String @external
            }

            type Query {
              productInBar: Product
            }
        "#, version)),
          service("baz", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Product @key(fields: "publicId fooCriteria") {
              publicId: ID!
              fooCriteria: String
              available: Boolean!
            }

            type Query {
              productInBaz: Product
            }
          "#, version)),
          service("qux", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@inaccessible"])

            type Product @key(fields: "publicId fooCriteria") {
              publicId: ID!
              fooCriteria: String @inaccessible
            }

            type Query {
              productInQux: Product
            }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        // Query.productInBar (bar) can't resolve Product.publicId (foo, baz, qux)
        // because it's marked as @external
        // `if (api.library === "apollo") { .. } else { .. }`: the guild (else) branch.
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message(normalize_error_message!(r#"
          The following supergraph API query:
          {
            productInBar {
              publicId
            }
          }
          cannot be satisfied by the subgraphs because:
          - from subgraph "bar":
            - field "Product.publicId" is not resolvable because marked @external.
            - cannot move to subgraph "qux" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
            - cannot move to subgraph "foo" using @key(fields: "key { code type } fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
            - cannot move to subgraph "foo" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
            - cannot move to subgraph "baz" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
          "#)).code("SATISFIABILITY_ERROR")]);

        // Query.productInBar (bar) can't resolve Product.available (baz)
        // Because:
        // - Product.available does not exist in subgraph "bar"
        // - can't move to other subgraphs that can resolve that field (foo, baz, qux)

        // `if (api.library === "apollo") { .. } else { .. }`: the guild (else) branch.
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                productInBar {
                  available
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "bar":
                - cannot find field "Product.available".
                - cannot move to subgraph "qux" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "foo" using @key(fields: "key { code type } fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "foo" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "baz" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
            "#)).code("SATISFIABILITY_ERROR")]);

        // `if (api.library === "apollo") { .. } else { .. }`: the guild (else) branch.
        assert_errors_contain(result.errors(), &[// Query.productInBar (bar) can't resolve Product.key (foo)
          // Because:
          // - Product.available key not exist in subgraph "bar"
          // - can't move to other subgraphs that can possibly resolve Product (foo, baz, qux) - not only the ones that can resolve Product.key, Product in general
          ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                productInBar {
                  key {
                    ...
                  }
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "bar":
                - cannot find field "Product.key".
                - cannot move to subgraph "qux" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "foo" using @key(fields: "key { code type } fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "foo" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "baz" using @key(fields: "publicId fooCriteria") of "Product", the key field(s) cannot be resolved from subgraph "bar".
            "#)).code("SATISFIABILITY_ERROR")]);

        result = api.compose_services(&[
          service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@inaccessible"])

            type Product @key(fields: "id") @key(fields: "id policy") {
              internalId: String! @inaccessible
              id: ID!
              policy: String
            }

            type Query {
              productInFoo: Product
            }
        "#, version)),
          service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            type Product @key(fields: "id policy") {
              id: ID! @external
              policy: String @external
            }

            type Query {
              productInBar: Product
            }
        "#, version)),
          service("env", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Product @key(fields: "id") {
              id: ID!
              environmentallyFriendly: Boolean!
            }

            type Query {
              productInBaz: Product
            }
          "#, version)),
          service("prices", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@inaccessible"])

            type Product @key(fields: "id policy") {
              id: ID!
              policy: String @inaccessible
            }

            type Query {
              productInQux: Product
            }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        // `if (api.library === "apollo") { .. } else { .. }`: the guild (else) branch.
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                productInBar {
                  id
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "bar":
                - field "Product.id" is not resolvable because marked @external.
                - cannot move to subgraph "products" using @key(fields: "id policy") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "products" using @key(fields: "id") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "prices" using @key(fields: "id policy") of "Product", the key field(s) cannot be resolved from subgraph "bar".
                - cannot move to subgraph "env" using @key(fields: "id") of "Product", the key field(s) cannot be resolved from subgraph "bar".
            "#)).code("SATISFIABILITY_ERROR")]);

        // `if (api.library === "apollo") { .. } else { .. }`: the guild (else) branch.
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
            The following supergraph API query:
            {
              productInBar {
                environmentallyFriendly
              }
            }
            cannot be satisfied by the subgraphs because:
            - from subgraph "bar":
              - cannot find field "Product.environmentallyFriendly".
              - cannot move to subgraph "products" using @key(fields: "id policy") of "Product", the key field(s) cannot be resolved from subgraph "bar".
              - cannot move to subgraph "products" using @key(fields: "id") of "Product", the key field(s) cannot be resolved from subgraph "bar".
              - cannot move to subgraph "prices" using @key(fields: "id policy") of "Product", the key field(s) cannot be resolved from subgraph "bar".
              - cannot move to subgraph "env" using @key(fields: "id") of "Product", the key field(s) cannot be resolved from subgraph "bar".
          "#)).code("SATISFIABILITY_ERROR")]);
    }

    // ADD IT TO THE COLLECTION
    fn external_but_somehow_resolvable(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("products", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Product @key(fields: "id category")  {
              id: ID!
              categoryDescription: String
              category: String
            }

            type Query {
              productInProducts: Product
            }
        "#, version)),
            service("bar", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            type Product @key(fields: "id stockId") {
              id: ID!
              freeBar: Boolean!
              stockId: String
            }

            type Query {
              productInBar: Product
            }
        "#, version)),
            service("env", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Product @key(fields: "id")  {
              id: ID!
              environmentallyFriendly: String!
            }

            type Query {
              productInEnv: Product
            }
          "#, version)),
            service("random", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            extend type Product @key(fields: "id") {
              id: ID! @external
              randomThing: String
            }

            type Query {
              productInRandom: Product
            }

            "#, version)),
            service("prices", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            extend type Product @key(fields: "id stockId")  {
              id: ID! @external
              stockId: String
              price: Float
            }

            type Query {
              productInPrices: Product
            }
            "#, version)),
            service("base", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            type Product @key(fields: "id") @key(fields: "id stockId") @key(fields: "id category") {
              id: ID!
              stockId: String
              category: String
            }

            type Query {
              productInBase: Product
            }
            "#, version)),
          ]),
        );
    }

    fn field_resolvable_via_entity_type_s_child(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("foo", graphql!(r#"
            type Query {
              foo: Foo
            }

            type Foo {
              user: User
            }

            type User @key(fields: "id profile { id }") @extends {
              id: ID!
              profile: Profile! @external
            }

            type Profile @extends {
              id: ID!
            }
          "#)),
            service("bar", graphql!(r#"
            type Query {
              bar: Bar
            }

            type Bar {
              user: User
            }

            type User @key(fields: "id profile { id }") {
              id: ID!
              profile: Profile!
            }

            type Profile {
              id: ID!
              name: String
            }
          "#)),
          ]),
        );
    }

    fn fed_v1_same_types_but_extra_root_field_in_a_subgraph_missing_a_field(api, version) {
        assert_errors_equal(
          api.compose_services(&[
            service("a", graphql!(r#"
            type Query {
              userById(id: ID): User
              randomUser: User # this is not in subgraph "a"
            }

            type User {
              id: ID!
              name: String!
            }
          "#)),
            service("b", graphql!(r#"
            type Query {
              userById(id: ID): User
            }

            type User {
              id: ID!
              name: String!
              nickname: String # this is not in subgraph "a"
            }
          "#)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  randomUser {\n",
                    "    nickname\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"a\":\n",
                    "  - cannot find field \"User.nickname\".\n",
                    "  - cannot move to subgraph \"b\", which has field \"User.nickname\", because type \"User\" has no @key defined in subgraph \"b\".",
                  )).code("SATISFIABILITY_ERROR"),
          ],
        );
    }

    fn fed_v1_same_types_but_extra_root_field_in_a_subgraph_defining_extra_field(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            type Query {
              userById(id: ID): User
            }

            type User {
              id: ID!
              name: String!
              # missing nickname
            }
          "#)),
            service("b", graphql!(r#"
            type Query {
              userById(id: ID): User
              randomUser: User # this is not in subgraph "a"
            }

            type User {
              id: ID!
              name: String!
              nickname: String # this is not in subgraph "a"
            }
          "#)),
            service("c", graphql!(r#"
            type Query {
              userById(id: ID): User
              randomUser: User # this is not in subgraph "a"
            }

            type User {
              id: ID!
              name: String!
              # missing nickname
            }
          "#)),
          ]),
        );
    }

    fn fed_v1_same_root_fields_same_types_additional_fields(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            type Query {
              userById(id: ID): User
            }

            type Mutation {
              createUser(name: String!): User
            }

            type User {
              id: ID!
              name: String!
            }
          "#)),
            service("b", graphql!(r#"
            type Query {
              userById(id: ID): User
            }

            type User {
              id: ID!
              name: String!
              nickname: String # this is not in subgraph "a"
            }

            type Mutation {
              createUser(name: String!): User
            }
          "#)),
          ]),
        );
    }

    fn moving_between_entity_with_key_and_without_it(api, version) {
        let result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

          extend type User @key(fields: "id") {
            id: ID @external
            email: String
          }

          type Query {
            usersById(ids: [ID!]!): [User!]!
          }
        "#, version)),
          service("b", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

          type User {
            profile: String
          }

          union Account = User | Admin

          type Admin {
            id: ID
            photo: String
          }

          type Query {
            accounts(ids: [ID!]!): [Account!]!
          }
        "#, version)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
          The following supergraph API query:
          {
            usersById(ids: []) {
              profile
            }
          }
          cannot be satisfied by the subgraphs because:
          - from subgraph "a":
            - cannot find field "User.profile".
            - cannot move to subgraph "b", which has field "User.profile", because type "User" has no @key defined in subgraph "b".
        "#)).code("SATISFIABILITY_ERROR")]);

        // `if (api.library === "apollo") { .. }`: Apollo-only assertions, not ported.
    }

    fn gateway_can_move_from_one_graph_to_another_through_other_subgraphs(api, version) {
        let result = api.compose_services(&[
          service("foo", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

          type Product @key(fields: "id category")  {
            id: ID!
            categoryDescription: String
            category: String
          }

          type Query {
            productInFoo: Product
          }
      "#, version)),
          service("bar", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

          type Product @key(fields: "id stockId") {
            id: ID!
            freeBar: Boolean!
            stockId: String
          }

          type Query {
            productInBar: Product
          }
      "#, version)),
          service("baz", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

          type Product @key(fields: "id")  {
            id: ID!
            environmentFriendly: String!
          }

          type Query {
            productInBaz: Product
          }
        "#, version)),
          service("qux", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

          extend type Car @key(fields: "id") {
            id: ID! @external
            randomThing: String
          }

          type Query {
            carInQux: Car
          }

          "#, version)),
          service("qux2", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

          extend type Car @key(fields: "id stockId")  {
            id: ID! @external
            stockId: String
            price: Float
          }

          type Query {
            carInQux2: Car
          }
          "#, version)),
          service("qux3", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

          type Car @key(fields: "id") @key(fields: "id stockId") @key(fields: "id category") {
            id: ID!
            stockId: String
            category: String
          }

          type Query {
            carInQux3: Car
          }
          "#, version)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                productInFoo {
                  freeBar
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "foo":
                - cannot find field "Product.freeBar".
                - cannot move to subgraph "bar" using @key(fields: "id stockId") of "Product", the key field(s) cannot be resolved from subgraph "foo".
            "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                productInFoo {
                  stockId
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "foo":
                - cannot find field "Product.stockId".
                - cannot move to subgraph "bar" using @key(fields: "id stockId") of "Product", the key field(s) cannot be resolved from subgraph "foo".
          "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
          The following supergraph API query:
          {
            productInBaz {
              freeBar
            }
          }
          cannot be satisfied by the subgraphs because:
          - from subgraph "baz":
            - cannot find field "Product.freeBar".
            - cannot move to subgraph "foo" using @key(fields: "id category") of "Product", the key field(s) cannot be resolved from subgraph "baz".
            - cannot move to subgraph "bar" using @key(fields: "id stockId") of "Product", the key field(s) cannot be resolved from subgraph "baz".
        "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
            The following supergraph API query:
            {
              productInBaz {
                stockId
              }
            }
            cannot be satisfied by the subgraphs because:
            - from subgraph "baz":
              - cannot find field "Product.stockId".
              - cannot move to subgraph "foo" using @key(fields: "id category") of "Product", the key field(s) cannot be resolved from subgraph "baz".
              - cannot move to subgraph "bar" using @key(fields: "id stockId") of "Product", the key field(s) cannot be resolved from subgraph "baz".
            "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
          The following supergraph API query:
          {
            productInBaz {
              categoryDescription
            }
          }
          cannot be satisfied by the subgraphs because:
          - from subgraph "baz":
            - cannot find field "Product.categoryDescription".
            - cannot move to subgraph "foo" using @key(fields: "id category") of "Product", the key field(s) cannot be resolved from subgraph "baz".
            - cannot move to subgraph "bar" using @key(fields: "id stockId") of "Product", the key field(s) cannot be resolved from subgraph "baz".
        "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                productInBaz {
                  category
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "baz":
                - cannot find field "Product.category".
                - cannot move to subgraph "foo" using @key(fields: "id category") of "Product", the key field(s) cannot be resolved from subgraph "baz".
                - cannot move to subgraph "bar" using @key(fields: "id stockId") of "Product", the key field(s) cannot be resolved from subgraph "baz".
          "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
              The following supergraph API query:
              {
                productInBar {
                  categoryDescription
                }
              }
              cannot be satisfied by the subgraphs because:
              - from subgraph "bar":
                - cannot find field "Product.categoryDescription".
                - cannot move to subgraph "foo" using @key(fields: "id category") of "Product", the key field(s) cannot be resolved from subgraph "bar".
            "#)).code("SATISFIABILITY_ERROR")]);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(normalize_error_message!(r#"
            The following supergraph API query:
            {
              productInBar {
                categoryDescription
              }
            }
            cannot be satisfied by the subgraphs because:
            - from subgraph "bar":
              - cannot find field "Product.categoryDescription".
              - cannot move to subgraph "foo" using @key(fields: "id category") of "Product", the key field(s) cannot be resolved from subgraph "bar".
          "#)).code("SATISFIABILITY_ERROR")]);
    }

    fn cannot_move_subgraphs_without_key_and_common_query_path(api, version) {
        assert_errors_contain(
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            type User {
              id: String!
              name: String!
            }

            type Query {
              users: [User]
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            extend type User {
              tags: [Tag]
            }

            type Tag {
              id: String!
              name: String!
            }
          "#, version)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  users {\n",
                    "    tags {\n",
                    "      ...\n",
                    "    }\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"a\":\n",
                    "  - cannot find field \"User.tags\".\n",
                    "  - cannot move to subgraph \"b\", which has field \"User.tags\", because type \"User\" has no @key defined in subgraph \"b\".",
                  )),
          ],
        );

        // Federation v1
        assert_errors_contain(
          api.compose_services(&[
            service("a", graphql!(r#"
            type User {
              id: String!
              name: String!
            }

            type Query {
              users: [User]
            }
          "#)),
            service("b", graphql!(r#"
            extend type User {
              tags: [Tag]
            }

            type Tag {
              id: String!
              name: String!
            }
          "#)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                    "The following supergraph API query:\n",
                    "{\n",
                    "  users {\n",
                    "    tags {\n",
                    "      ...\n",
                    "    }\n",
                    "  }\n",
                    "}\n",
                    "cannot be satisfied by the subgraphs because:\n",
                    "- from subgraph \"a\":\n",
                    "  - cannot find field \"User.tags\".\n",
                    "  - cannot move to subgraph \"b\", which has field \"User.tags\", because type \"User\" has no @key defined in subgraph \"b\".",
                  )),
          ],
        );
    }

    fn from_root_field_subgraph_a_to_entity_subgraph_a_b_with_missing_key_b(api, version) {
        // Even though key field fields are different, { user { nickname } } still can be resolved.
        // We cannot ask subgraph A, to resolve Query.user, as it's not defined there.
        // We need to ask subgraph B, but it can't resolve User.nickname, as it's not defined there.
        // We can't move to subgraph A by using @key(fields: "id"), but we can extend the selection set to include `email`.
        // This way we get the `email` and resolve `nickname` in subgraph B by making a call to `Query._entities` with the "email" as the key.
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

            type Query {
              user: User
            }

            type User @key(fields: "id") {
              id: ID!
              email: String!
            }
          "#, version)),
            service("b", graphql!(r#"
            extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

            type User @key(fields: "email") {
              email: String! @external
              nickname: String!
            }
          "#, version)),
          ]),
        );

        assert_errors_contain(
          api.compose_services(&[
            service("a", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key"])

          type Query {
            user: User
          }

          type User @key(fields: "id") {
            id: ID!
            name: String!
          }
        "#, version)),
            service("b", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

          type User @key(fields: "email") {
            email: String! @external
            nickname: String!
          }
        "#, version)),
            service("c", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@external"])

          type User @key(fields: "email") {
            email: String!
            country: String!
          }
        "#, version)),
          ]),
          &[
              ErrorMatcher::new().message_containing(concat!(
                        "The following supergraph API query:\n",
                        "{\n",
                        "  user {\n",
                        "    email\n",
                        "  }\n",
                        "}\n",
                        "cannot be satisfied by the subgraphs because:\n",
                        "- from subgraph \"a\":\n",
                        "  - cannot find field \"User.email\".\n",
                        "  - cannot move to subgraph \"c\" using @key(fields: \"email\") of \"User\", the key field(s) cannot be resolved from subgraph \"a\".\n",
                        "  - cannot move to subgraph \"b\" using @key(fields: \"email\") of \"User\", the key field(s) cannot be resolved from subgraph \"a\".",
                      )).code("SATISFIABILITY_ERROR"),
          ],
        );
    }

    fn resolve_missing_field_by_parent_entity(api, version) {
        let result = api.compose_services(&[
          service("product", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@external"])

          type Query {
            products: [Product!]!
          }

          type Product @key(fields: "id") @key(fields: "id pid") {
            id: ID!
            pid: ID
          }

          type Category @key(fields: "id name") {
            id: ID!
            name: String! @external
          }
        "#, version)),
          service("product-category", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type Product @key(fields: "id pid") {
            id: ID!
            pid: ID
            category: Category
          }

          type Category @key(fields: "id name") @key(fields: "cid") {
            id: ID!
            cid: ID!
            name: String!
          }
        "#, version)),
          service("category", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type Category @key(fields: "id cid") {
            id: ID!
            cid: ID
            details: CategoryDetails
          }

          type CategoryDetails {
            products: Int
          }
        "#, version)),
        ]);

        // It's possible to resolve Category.details only because it can be resolved by the parent entity (Car.category).
        assert_composition_success(&result);
    }

    fn resolve_missing_field_by_resolving_first_deeply_nested_key_fields_from_multiple_subgraphs(api, version) {
        let result = api.compose_services_with_internal(
          &[
            service("products", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: ["@key", "@external", "@extends", "@shareable"]
            )

          type Query {
            topProducts: ProductList!
          }

          type ProductList @key(fields: "products{id}") {
            products: [Product!]!
          }

          type Product @extends @key(fields: "id") {
            id: String! @external
            category: Category @shareable
          }

          type Category @key(fields: "id") {
            mainProduct: Product! @shareable
            id: String!
            tag: String @shareable
          }
        "#, version)),
            service("core", graphql!(r#"
          extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])

          type Product @key(fields: "id") @key(fields: "id pid") {
            id: String!
            pid: String!
          }
        "#, version)),
            service("product-list", graphql!(r#"
          extend schema
            @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

          type ProductList @key(fields: "products{id pid}") {
            products: [Product!]!
            first: Product @shareable
            last: Product @shareable
          }

          type Product @key(fields: "id pid") {
            id: String!
            pid: String
          }
        "#, version)),
            service("product-price", graphql!(r#"
            extend schema
              @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key", "@shareable"])

            type ProductList @key(fields: "products{id pid category{id tag}} last{id}") {
              products: [Product!]!
              first: Product @shareable
              last: Product @shareable
            }

            type Product @key(fields: "id pid category{id tag}") {
              id: String!
              price: Price
              pid: String
              category: Category
            }

            type Category @key(fields: "id tag") {
              id: String!
              tag: String
            }

            type Price {
              price: Float!
            }
          "#, version)),
          ],
          Some(InternalOptions::default()),
        );

        assert_composition_success(&result);
    }

    fn ignore_overridden_root_fields(api, version) {
        let result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: [
                "@shareable"
              ]
            )

          type Query {
            data: Data
          }

          type Data {
            id: String
            name: String
            notInA: String
            shared: String @shareable
          }
        "#, version)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/${version}"
              import: [
                "@override",
                "@shareable"
              ]
            )

          type Query {
            data: Data @override(from: "a")
          }

          type Data {
            id: String @override(from: "a")
            name: String @override(from: "a")
            shared: String @shareable
          }
        "#, version)),
        ]);

        assert_composition_failure(&result);

        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(concat!(
                "The following supergraph API query:\n",
                "{\n",
                "  data {\n",
                "    notInA\n",
                "  }\n",
                "}\n",
                "cannot be satisfied by the subgraphs because:\n",
                "- from subgraph \"b\":\n",
                "  - cannot find field \"Data.notInA\".\n",
                "  - cannot move to subgraph \"a\", which has field \"Data.notInA\", because type \"Data\" has no @key defined in subgraph \"a\".",
              )).code("SATISFIABILITY_ERROR")]);
    }

    fn make_sure_we_avoid_infinite_loops(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("foo", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@shareable", "@external"])

              type Query {
                node(id: ID!): Node @shareable
                account(id: String!): Account
              }

              interface Node {
                id: ID!
              }

              type Account implements Node @key(fields: "id") {
                id: ID!
                username: String!
              }

              type Chat implements Node @key(fields: "id") {
                id: ID! @external
                account: Account!
              }
            "#, version)),
            service("bar", graphql!(r#"
              extend schema @link(url: "https://specs.apollo.dev/federation/${version}" import: ["@key", "@shareable", "@external"])
              type Query {
                node(id: ID!): Node @shareable
                chat(id: String!): Chat
              }

              interface Node {
                id: ID!
              }

              type Account implements Node @key(fields: "id") {
                id: ID! @external
                chats: [Chat!]!
              }

              type Chat implements Node @key(fields: "id") {
                id: ID!
                text: String!
              }
          "#, version)),
          ]),
        );
    }

    fn fed1_external_and_extends_extend(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            type Query {
              randomUser: User
            }

            extend type User @key(fields: "id") {
              id: ID! @external
            }

            extend type User {
              name: String! @external
            }
          "#)),
            service("b", graphql!(r#"
            type Query {
              userById(id: ID): User
            }

            type User @key(fields: "id") {
              id: ID!
              name: String!
              nickname: String
            }
          "#)),
          ]),
        );

        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            type Query {
              randomUser: User
            }

            type User @extends {
              id: ID! @external
              name: String! @external
            }
          "#)),
            service("b", graphql!(r#"
            type Query {
              userById(id: ID): User
            }

            type User @key(fields: "id") {
              id: ID!
              name: String!
              nickname: String
            }
          "#)),
          ]),
        );
    }

    fn requires_with_interface_field(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            type Query {
              userFromA(id: ID): User
            }

            interface Address {
              id: ID!
            }

            type HomeAddress implements Address @key(fields: "id") {
              id: ID!
              city: String
            }

            type WorkAddress implements Address @key(fields: "id") {
              id: ID!
              city: String
            }

            type User @key(fields: "id") {
              id: ID!
              name: String!
              address: Address @external
              city: String @requires(fields: "address { id }")
            }
          "#)),
            service("b", graphql!(r#"
            type Query {
              userFromB(id: ID): User
            }

            interface Address {
              id: ID!
            }

            type HomeAddress implements Address @key(fields: "id") {
              id: ID!
              city: String
            }

            type WorkAddress implements Address @key(fields: "id") {
              id: ID!
              city: String
            }

            type User @key(fields: "id") {
              id: ID!
              name: String!
              address: Address
              city: String @requires(fields: "address { id }")
            }
          "#)),
          ]),
        );
    }

    fn requires_with_a_lot_of_nested_entities(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.0"
                import: ["@key"]
              )

            type Publisher @key(fields: "id", resolvable: false) {
              id: ID!
            }

            type Details @key(fields: "id") {
              publisher: Publisher!
              id: ID!
            }
          "#)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.0"
                import: ["@key"]
              )

            type Publisher @key(fields: "id") {
              id: ID!
              books: [Book!]!
            }

            type Book @key(fields: "id", resolvable: false) {
              id: ID!
            }
          "#)),
            service("c", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.0"
                import: ["@key"]
              )

            type Book @key(fields: "id") {
              id: ID!
              genres: [Genre!]!
            }

            type Genre {
              details: Details!
              name: String
            }

            type Details @key(fields: "id") {
              id: ID!
            }
          "#)),
            service("d", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.0"
                import: ["@key", "@requires", "@external"]
              )

            type Query {
              noop: String
            }

            type Mutation {
              order: Book!
            }

            type Genre {
              name: String @external
            }

            type Book @key(fields: "id") {
              id: ID!
              genres: [Genre!]! @external
              price: Float @requires(fields: "genres { name }")
            }
          "#)),
          ]),
        );

        let mut result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id isbn author{id name}") {
            id: ID!
            isbn: ID
            author: Author
          }

          type Author @key(fields: "id name") {
            id: ID!
            name: String
          }
        "#)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id") @key(fields: "id isbn title") {
            id: ID!
            isbn: ID
            title: String
          }
        "#)),
          service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@external", "@shareable"]
            )

          type Author @key(fields: "id") {
            id: ID!
            name: String @shareable
          }

          type Book @key(fields: "id") {
            author: Author @shareable
            id: ID! @external
          }

          type Query {
            books: [Book]!
          }
        "#)),
        ]);

        assert_composition_failure(&result);

        assert!(result.errors().unwrap()[0].message.contains(r#""Book.id""#));

        result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id isbn author{id name}") {
            id: ID!
            isbn: ID
            author: Author
          }

          type BookList
            @key(fields: "books{id isbn author{id name nickname}} last{id}") {
            books: [Book!]!
            first: Book @shareable
            last: Book @shareable
            related: BookList!
          }

          type Author @key(fields: "id name") {
            id: ID!
            name: String
            nickname: String
          }
        "#)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book
            @key(fields: "id")
            @key(fields: "id isbn title")
            @key(fields: "id publisher") {
            id: ID!
            isbn: ID
            title: String
            publisher: String
          }

          type BookList
            @key(fields: "books{id publisher} last{id}")
            @key(fields: "books{id isbn title}") {
            books: [Book!]!
            first: Book @shareable
            groupBy: [BookList!]!
            last: Book @shareable
          }
        "#)),
          service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@external", "@shareable"]
            )

          type Author @key(fields: "id") {
            id: ID!
            name: String @shareable
          }

          type Book @key(fields: "id") {
            author: Author @shareable
            id: ID! @external
          }

          type BookList @key(fields: "books{id}") {
            books: [Book!]!
            first: Book @shareable
          }

          type Query {
            books: [Book]!
          }
        "#)),
        ]);

        assert_composition_failure(&result);
        assert!(result.errors().unwrap()[0].message.contains(r#""Book.id""#));

        result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id title isbn") {
            id: ID!
            title: String
            isbn: String
          }

          type BookList @key(fields: "books{id title isbn}") {
            books: [Book!]!
            last: Book @shareable
          }
        "#)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id isbn author{id name country}") {
            id: ID!
            isbn: String
            author: Author
          }

          type BookList
            @key(fields: "books{id isbn author{id name country}} last{id}") {
            books: [Book!]!
            last: Book @shareable
          }

          type Author @key(fields: "id name country") {
            name: String!
            country: String
            id: ID
          }
        "#)),
          service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book
            @key(fields: "id")
            @key(fields: "id publisher")
            @key(fields: "id title isbn") {
            id: ID!
            publisher: String
            title: String
            isbn: String
          }

          type BookList @key(fields: "books{id title isbn}") {
            books: [Book!]!
            fav: Book
            last: Book @shareable
          }
        "#)),
          service("d", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@external", "@extends", "@shareable"]
            )

          type Author @key(fields: "id name") {
            name: String!
            country: String @shareable
            id: ID!
          }

          type Book @extends @key(fields: "id") {
            author: Author @shareable
            id: ID! @external
          }

          type BookList @key(fields: "books{id}") {
            books: [Book!]!
          }

          type Query {
            allBooks: BookList!
          }
        "#)),
        ]);

        assert_composition_failure(&result);
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(r#""BookList.fav""#)]);
        assert_eq!(result.errors().unwrap().len(), 1);

        result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id title isbn") {
            id: ID!
            title: String
            isbn: String
          }

          type BookList @key(fields: "books{id title isbn}") {
            books: [Book!]!
            last: Book @shareable
          }
        "#)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id isbn author{id name country}") {
            id: ID!
            isbn: String
            author: Author
          }

          type BookList
            @key(fields: "books{id isbn author{id name country}} last{id}") {
            books: [Book!]!
            last: Book @shareable
          }

          type Author @key(fields: "id name country") {
            name: String!
            country: String
            id: ID
          }
        "#)),
          service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book
            @key(fields: "id")
            @key(fields: "id publisher")
            @key(fields: "id title isbn") {
            id: ID!
            publisher: String
            title: String
            isbn: String
          }

          type BookList
            @key(fields: "books{id publisher} last{id}")
            @key(fields: "books{id title isbn}") {
            books: [Book!]!
            fav: Book
            last: Book @shareable
          }
        "#)),
          service("d", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@external", "@extends", "@shareable"]
            )

          type Author @key(fields: "id name") {
            name: String!
            country: String @shareable
            id: ID!
          }

          type Book @extends @key(fields: "id") {
            author: Author @shareable
            id: ID! @external
          }

          type BookList @key(fields: "books{id}") {
            books: [Book!]!
          }

          type Query {
            books: BookList!
          }
        "#)),
        ]);

        assert_composition_failure(&result);
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(r#""BookList.fav""#)]);
        assert_eq!(result.errors().unwrap().len(), 1);

        result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book @key(fields: "id title isbn") {
            id: ID!
            title: String
            isbn: String
          }

          type BookList @key(fields: "books{id title isbn}") {
            books: [Book!]!
            last: Book @shareable
          }
        "#)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@external", "@extends", "@shareable"]
            )

          type Book @extends @key(fields: "id publisher") {
            id: ID! @external
            publisher: String
          }

          type BookList @key(fields: "books{id publisher} last{id}") {
            books: [Book!]!
            last: Book @shareable
          }
        "#)),
          service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@shareable"]
            )

          type Book
            @key(fields: "id")
            @key(fields: "id publisher")
            @key(fields: "id title isbn") {
            id: ID!
            publisher: String
            title: String
            isbn: String
          }

          type BookList
            @key(fields: "books{id publisher} last{id}")
            @key(fields: "books{id title isbn}") {
            books: [Book!]!
            fav: Book
            last: Book @shareable
          }
        "#)),
          service("d", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.0"
              import: ["@key", "@external", "@extends", "@shareable"]
            )

          type Author @key(fields: "id name") {
            name: String!
            country: String @shareable
            id: String!
          }

          type Book @extends @key(fields: "id") {
            author: Author @shareable
            id: ID! @external
          }

          type BookList @key(fields: "books{id}") {
            books: [Book!]!
          }

          type Query {
            allBooks: BookList!
          }
        "#)),
        ]);

        assert_composition_failure(&result);
        assert_errors_contain(result.errors(), &[ErrorMatcher::new().message_containing(r#""BookList.fav""#)]);
        assert_eq!(result.errors().unwrap().len(), 1);
    }

    fn require_in_interface_object(api, version) {
        let result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@interfaceObject"]
            )

          type Product @key(fields: "id") @interfaceObject {
            id: String!
            category: Category!
          }

          type Category @key(fields: "id") {
            id: ID!
            name: String!
          }
        "#)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@interfaceObject", "@requires", "@external"]
            )

          type Product @key(fields: "id") @interfaceObject {
            id: String!
            categoryName: String @requires(fields: "category { name }")
            category: Category! @external
          }

          type Category {
            name: String! @external
          }
        "#)),
          service("c", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@interfaceObject"]
            )

          interface Product @key(fields: "id") {
            id: String!
          }

          type Tile implements Product @key(fields: "id") {
            id: String!
            size: Int!
          }

          type Query {
            tiles: [Tile!]!
          }
        "#)),
        ]);

        assert_composition_success(&result);
    }

    fn deeply_nested_interfaces_with_requires_fields_fragment(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@external", "@requires"]
              )

            interface Product {
              id: ID!
            }

            interface Location {
              id: ID!
            }

            type Warsaw implements Location @key(fields: "id") {
              id: ID!
              products: [Product!]! @external
            }

            type Lidl @key(fields: "id") {
              id: ID!
              locations: [Location!] @external
              products(location: ID): [Product]!
                @requires(
                  fields: """
                  locations {
                    ... on Warsaw {
                      products {
                        id
                      }
                    }
                  }
                  """
                )
            }
          "#)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key"]
              )

            type Query {
              products: [Product]!
            }

            interface Shop @key(fields: "id") {
              id: ID!
            }

            interface Location @key(fields: "id") {
              id: ID!
            }

            type Lidl implements Shop @key(fields: "id") {
              id: ID!
              locations: [Location!]
            }

            type Warsaw implements Location @key(fields: "id") {
              id: ID!
              products: [Product!]!
            }

            interface Product @key(fields: "id") {
              id: ID!
            }

            type Bread implements Product @key(fields: "id") {
              id: ID!
              shops: [Shop!]!
            }
          "#)),
          ]),
        );
    }

    fn unreachable_interface_implementation_interface_object_interface_lacks_key(api, version) {
        // This is a valid composition, as the interface is annotated with @key
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key"]
              )

            interface IData @key(fields: "id") {
              id: String
            }

            type Data implements IData @key(fields: "id") {
              id: String
            }
          "#)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type Query {
              idata: IData
            }

            type IData @interfaceObject @key(fields: "id") {
              id: String
            }
          "#)),
          ]),
        );

        // This is an invalid composition, as the interface is NOT annotated with @key
        let result = api.compose_services(&[
          service("a", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key"]
            )

          interface IData {
            id: String
          }

          type Data implements IData @key(fields: "id") {
            id: String
          }
        "#)),
          service("b", graphql!(r#"
          extend schema
            @link(
              url: "https://specs.apollo.dev/federation/v2.3"
              import: ["@key", "@interfaceObject"]
            )

          type Query {
            idata: IData
          }

          type IData @interfaceObject @key(fields: "id") {
            id: String
          }
        "#)),
        ]);
        assert_composition_failure(&result);

        // `expect.stringMatching(message)` makes a RegExp of the message; it has no regex syntax other
        // than `.` (any character but a line terminator), so `matches_js_regex_with_dots` is that RegExp.
        let pattern = normalize_error_message!(r#"
          The following supergraph API query:
          {
            idata {
              ... on Data {
                ...
              }
            }
          }
          cannot be satisfied by the subgraphs because:
          - from subgraph "b": no subgraph can be reached to resolve the implementation type of @interfaceObject type "IData"."#);
        assert!(
          result.errors().unwrap().iter().any(|error| {
            matches_js_regex_with_dots(&error.message, &pattern) && error.code() == Some("SATISFIABILITY_ERROR")
          }),
          "no error matches /{pattern}/",
        );
    }

    fn make_sure_moving_from_interface_object_to_an_implementation_of_the_interface_is_allowed(api, version) {
        assert_composition_success(&
          api.compose_services(&[
            service("a", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key", "@interfaceObject"]
              )

            type Query {
              products: Product!
            }

            type Product
              @key(fields: "id", resolvable: false)
              @interfaceObject {
              id: ID!
            }
          "#)),
            service("b", graphql!(r#"
            extend schema
              @link(
                url: "https://specs.apollo.dev/federation/v2.3"
                import: ["@key"]
              )

            interface Product @key(fields: "id") {
              id: ID!
            }

            type Bread implements Product @key(fields: "id") {
              id: ID!
              name: String!
            }
          "#)),
          ]),
        );
    }
}
