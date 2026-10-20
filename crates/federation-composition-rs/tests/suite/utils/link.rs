//! Port of `__tests__/utils/link.spec.ts`.

use crate::shared::*;

use federation_composition_rs::FederatedLink;

/// `trimMultiline(str)`.
fn trim_multiline(str: &str) -> String {
    str.split('\n').map(js_trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

mod federated_link {
    use super::*;

    /// The body of the `fromTypedefs` `test.each`.
    fn assert_from_typedefs(sdl: &str) {
        // string manipulate to extract just the link trings
        let first_link_pos = sdl.find("@link(").expect("the sdl has a @link");
        let links_only = trim_multiline(&sdl[first_link_pos..]);
        // compare to parsed result
        let type_defs = parse(sdl).unwrap();
        let links = FederatedLink::from_typedefs(&type_defs).unwrap();
        let joined = links.iter().map(FederatedLink::to_string).collect::<Vec<_>>().join("\n");
        assert_eq!(joined, links_only);
    }

    #[test]
    fn from_typedefs() {
        assert_from_typedefs(
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/federation/v2.0")
    "#,
        );
    }

    #[test]
    fn from_typedefs_2() {
        assert_from_typedefs(
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0", as: "@linkz")
        @link(url: "https://specs.apollo.dev/federation/v2.0", as: "fed", import: ["@key"])
    "#,
        );
    }

    #[test]
    fn from_typedefs_3() {
        assert_from_typedefs(
            r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/federation/v2.0", import: [{ name: "@key", as: "@lookup" }])
    "#,
        );
    }

    #[test]
    fn resolve_import_name() {
        let sdl = r#"
      extend schema
        @link(url: "https://specs.apollo.dev/link/v1.0")
        @link(url: "https://specs.apollo.dev/federation/v2.0", import: [{ name: "@key", as: "@lookup" }, "@provides"])
        @link(url: "https://unnamed.graphql-hive.com/v0.1", import: ["@meta"])
        @link(url: "https://specs.graphql-hive.com/hive/v0.1", import: ["@group"], as: "hivelink")
    "#;
        let links = FederatedLink::from_typedefs(&parse(sdl).unwrap()).unwrap();
        let federation_link = links.iter().find(|l| l.identity() == "https://specs.apollo.dev/federation");
        assert!(federation_link.is_some());
        let federation_link = federation_link.unwrap();
        // aliased
        assert_eq!(federation_link.resolve_import_name("@key"), "lookup");
        // unimported
        assert_eq!(federation_link.resolve_import_name("@external"), "federation__external");
        // imported by name only
        assert_eq!(federation_link.resolve_import_name("@provides"), "provides");

        // default import
        let link_link = links.iter().find(|l| l.identity() == "https://specs.apollo.dev/link");
        assert_eq!(link_link.map(|l| l.resolve_import_name("@link")).as_deref(), Some("link"));

        // unnamespaced
        let unnamed_link = links.iter().find(|l| l.identity() == "https://unnamed.graphql-hive.com");
        assert!(unnamed_link.is_some());
        let unnamed_link = unnamed_link.unwrap();
        assert_eq!(unnamed_link.resolve_import_name("@meta"), "meta");
        assert_eq!(unnamed_link.resolve_import_name("@unmentioned"), "unmentioned");

        // imported as
        let hive_link = links.iter().find(|l| l.identity() == "https://specs.graphql-hive.com/hive");
        assert_eq!(hive_link.map(|l| l.resolve_import_name("@group")).as_deref(), Some("group"));
        assert_eq!(hive_link.map(|l| l.resolve_import_name("@eg")).as_deref(), Some("hivelink__eg"));
    }
}
