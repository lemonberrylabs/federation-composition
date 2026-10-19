//! Port of `src/specifications/join.ts`.

use super::federation::FederationVersion;
use crate::utils::version::satisfies_version_range;

/// `sdl(version)`: the join spec for a federation version.
pub fn sdl(version: FederationVersion) -> String {
    let join_field = format!(
        r#"
      directive @join__field(
        graph: join__Graph
        requires: join__FieldSet
        provides: join__FieldSet
        type: String
        external: Boolean
        override: String
        usedOverridden: Boolean
        {override_label}
        {context_arguments}
      ) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION
    "#,
        override_label = if satisfies_version_range(version, ">= v2.7") {
            "overrideLabel: String"
        } else {
            ""
        },
        context_arguments = if satisfies_version_range(version, ">= v2.8") {
            "contextArguments: [join__ContextArgument!]"
        } else {
            ""
        },
    );

    let join_context_argument = if satisfies_version_range(version, ">= v2.8") {
        r#"
        input join__ContextArgument {
          name: String!
          type: String!
          context: String!
          selection: join__FieldValue!
        }
      "#
    } else {
        ""
    };

    let join_field_value = if satisfies_version_range(version, ">= v2.8") {
        r#"
        scalar join__FieldValue
      "#
    } else {
        ""
    };

    format!(
        r#"
    directive @join__enumValue(graph: join__Graph!) repeatable on ENUM_VALUE

    directive @join__graph(name: String!, url: String!) on ENUM_VALUE

    {join_field}
    {join_context_argument}

    directive @join__implements(
      graph: join__Graph!
      interface: String!
    ) repeatable on OBJECT | INTERFACE

    directive @join__type(
      graph: join__Graph!
      key: join__FieldSet
      extension: Boolean! = false
      resolvable: Boolean! = true
      isInterfaceObject: Boolean! = false
    ) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR

    directive @join__unionMember(
      graph: join__Graph!
      member: String!
    ) repeatable on UNION

    scalar join__FieldSet
    {join_field_value}
  "#
    )
}
