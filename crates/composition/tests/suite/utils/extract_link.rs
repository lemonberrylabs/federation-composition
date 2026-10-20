//! Port of `__tests__/utils/extract-link.spec.ts`.

use crate::shared::*;

use federation_composition::{extract_link_implementations, LinkImplementations, LinkVersion};
use graphql_js::{visit, NodeRef, Value, VisitAction, VisitInfo, Visitor};
use serde_json::{Map, Value as Json};

mod extract_link_implementations {
    use super::*;

    const SDL: &str = r#"
    directive @meta(name: String!, content: String!) on SCHEMA | FIELD
    directive @metadata__example(eg: String!) on FIELD
    extend schema
      @link(url: "https://specs.apollo.dev/federation/v2.3")
      @link(url: "https://specs.graphql-hive.com/metadata/v0.1", import: ["@meta"])
      @link(url: "https://specs.graphql-hive.com/foo/v1.1")

    type Query {
      ping: String @meta(name: "owner", content: "hive-console-team")
      pong: String @metadata__example(eg: "1...2...3... Pong")
    }
  "#;

    /// `const typeDefs = parse(sdl)`.
    fn type_defs() -> Document {
        parse(SDL).unwrap()
    }

    /// `const { matchesImplementation, resolveImportName } = extractLinkImplementations(typeDefs)`.
    fn implementations() -> LinkImplementations {
        extract_link_implementations(&type_defs()).unwrap()
    }

    fn v(version: &str) -> LinkVersion {
        LinkVersion::Str(version.to_string())
    }

    #[test]
    fn can_match_alpha_v0_x_versions_correctly() {
        let links = implementations();
        assert!(links.matches_implementation("https://specs.graphql-hive.com/metadata", &v("v0.1")));
        assert!(!links.matches_implementation("https://specs.graphql-hive.com/metadata", &v("v0.2")));
    }

    #[test]
    fn can_match_non_alpha_v1_x_versions_correctly() {
        let links = implementations();
        assert!(!links.matches_implementation("https://specs.graphql-hive.com/metadata", &v("v1.0")));
        assert!(links.matches_implementation(
            "https://specs.graphql-hive.com/foo",
            &LinkVersion::MajorMinor { major: 1, minor: 0 }
        ));
        assert!(links.matches_implementation("https://specs.graphql-hive.com/foo", &v("v1.1")));
        assert!(!links.matches_implementation(
            "https://specs.graphql-hive.com/foo",
            &LinkVersion::MajorMinor { major: 1, minor: 2 }
        ));
        assert!(!links.matches_implementation(
            "https://specs.graphql-hive.com/foo",
            &LinkVersion::MajorMinor { major: 2, minor: 0 }
        ));
    }

    #[test]
    fn does_not_match_federation_v1_version_if_linked() {
        let links = implementations();
        assert!(!links.matches_implementation("https://specs.graphql-hive.com/metadata", &LinkVersion::FederationV1));
    }

    #[test]
    fn matches_federation_v1_version_if_no_federation_link_is_provided() {
        let sdl = r#"
      directive @meta(name: String!, content: String!) on SCHEMA | FIELD

      type Query {
        ping: String @meta(name: "owner", content: "hive-console-team")
      }
    "#;
        let type_defs = parse(sdl).unwrap();
        let links = extract_link_implementations(&type_defs).unwrap();
        assert!(links.matches_implementation("https://specs.graphql-hive.com", &LinkVersion::FederationV1));
        assert_eq!(links.resolve_import_name("https://specs.graphql-hive.com", "@meta"), "meta");
    }

    #[test]
    fn does_not_match_null_version_if_linked_to_a_specific_version() {
        let links = implementations();
        assert!(!links.matches_implementation("https://specs.graphql-hive.com/metadata", &LinkVersion::Null));
    }

    #[test]
    fn matches_null_version_if_link_does_not_include_a_specific_version() {
        let sdl = r#"
      directive @meta(name: String!, content: String!) on SCHEMA | FIELD
      extend schema
        @link(url: "https://specs.apollo.dev/federation/v2.3")
        @link(url: "https://specs.graphql-hive.com", import: ["@meta"])

      type Query {
        ping: String @meta(name: "owner", content: "hive-console-team")
      }
    "#;
        let type_defs = parse(sdl).unwrap();
        let links = extract_link_implementations(&type_defs).unwrap();
        assert!(links.matches_implementation("https://specs.graphql-hive.com", &LinkVersion::Null));
    }

    #[test]
    fn can_resolve_name_of_import() {
        let links = implementations();
        assert_eq!(links.resolve_import_name("https://specs.graphql-hive.com/metadata", "@meta"), "meta");
    }

    #[test]
    fn can_resolve_a_namespaced_import() {
        let links = implementations();
        assert_eq!(
            links.resolve_import_name("https://specs.graphql-hive.com/metadata", "@example"),
            "metadata__example"
        );
    }

    /// The `visit(typeDefs, { FieldDefinition })` visitor of "can be used to develop plugins".
    struct CollectMeta {
        meta_name: String,
        collected_meta: Map<String, Json>,
    }

    /// `(arg?.value as StringValueNode | undefined)?.value ?? "??"`.
    fn string_argument(value: Option<&Value>) -> String {
        match value {
            Some(Value::String(s)) => s.value.clone(),
            _ => "??".to_string(),
        }
    }

    impl<'a> Visitor<'a> for CollectMeta {
        fn enter(&mut self, node: NodeRef<'a>, _cx: &mut (), _info: &VisitInfo<'a, '_>) -> VisitAction {
            let NodeRef::FieldDefinition(node) = node else {
                return VisitAction::Continue;
            };
            let mut meta_data: Map<String, Json> = Map::new();
            let id = node.name.value.clone();
            let meta = node.directives.iter().find(|d| d.name.value == self.meta_name);
            if let Some(meta) = meta {
                let argument = |name: &str| meta.arguments.iter().find(|a| a.name.value == name).map(|a| &a.value);
                meta_data.insert("name".to_string(), Json::String(string_argument(argument("name"))));
                meta_data.insert("content".to_string(), Json::String(string_argument(argument("content"))));
            }
            if !meta_data.is_empty() {
                let entry = self.collected_meta.entry(id).or_insert_with(|| Json::Object(Map::new()));
                if let Json::Object(existing) = entry {
                    existing.extend(meta_data);
                }
            }
            VisitAction::Continue
        }
    }

    #[test]
    fn can_be_used_to_develop_plugins() {
        let type_defs = type_defs();
        let links = extract_link_implementations(&type_defs).unwrap();
        if links.matches_implementation("https://specs.graphql-hive.com/metadata", &v("v0.1")) {
            let meta_name = links.resolve_import_name("https://specs.graphql-hive.com/metadata", "@meta");
            let mut visitor = CollectMeta { meta_name, collected_meta: Map::new() };
            visit(&type_defs, &mut visitor, &mut ());
            to_match_inline_snapshot(
                Json::Object(visitor.collected_meta),
                r#"
        {
          "ping": {
            "content": "hive-console-team",
            "name": "owner",
          },
        }
      "#,
            );
        }
    }
}
