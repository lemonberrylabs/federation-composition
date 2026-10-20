//! Port of `__tests__/utils/link-url.spec.ts`.

use crate::shared::*;

use federation_composition_rs::{FederatedLinkUrl, SupportsVersion};

mod federated_link_url {
    use super::*;

    /// The body of "fromUrl correctly parses the identity, name, and version".
    fn from_url_parses(url: &str, identity: &str, name: Option<&str>, version: Option<&str>) {
        let spec = FederatedLinkUrl::from_url(url).unwrap();
        assert_eq!(spec.identity(), identity);
        assert_eq!(spec.name(), name);
        assert_eq!(spec.version(), version);
    }

    #[test]
    fn from_url_correctly_parses_the_identity_name_and_version() {
        from_url_parses(
            "https://spec.example.com/a/b/mySchema/v1.0/",
            "https://spec.example.com/a/b/mySchema",
            Some("mySchema"),
            Some("v1.0"),
        );
    }

    #[test]
    fn from_url_correctly_parses_the_identity_name_and_version_2() {
        from_url_parses("https://spec.example.com", "https://spec.example.com", None, None);
    }

    #[test]
    fn from_url_correctly_parses_the_identity_name_and_version_3() {
        from_url_parses(
            "https://spec.example.com/mySchema/v0.1?q=v#frag",
            "https://spec.example.com/mySchema",
            Some("mySchema"),
            Some("v0.1"),
        );
    }

    #[test]
    fn from_url_correctly_parses_the_identity_name_and_version_4() {
        from_url_parses(
            "https://spec.example.com/v1.0",
            "https://spec.example.com",
            None,
            Some("v1.0"),
        );
    }

    #[test]
    fn from_url_correctly_parses_the_identity_name_and_version_5() {
        from_url_parses(
            "https://spec.example.com/vX",
            "https://spec.example.com/vX",
            Some("vX"),
            None,
        );
    }

    #[test]
    fn from_url_correctly_parses_the_identity_name_and_version_6() {
        from_url_parses(
            "file:///extensions/permissions",
            "file:///extensions/permissions",
            Some("permissions"),
            None,
        );
    }

    #[test]
    fn from_url_errors_on_invalid_url_parses_the_identity_name_and_version() {
        assert_throws(
            FederatedLinkUrl::from_url("spec.example.com").map_err(JsError::from),
            Some("Invalid URL"),
        );
    }

    /// `FederatedLinkUrl.fromUrl(url0).supports(FederatedLinkUrl.fromUrl(url1))`.
    fn supports(url0: &str, url1: &str) -> bool {
        let other = FederatedLinkUrl::from_url(url1).unwrap();
        FederatedLinkUrl::from_url(url0)
            .unwrap()
            .supports(SupportsVersion::Url(&other))
    }

    #[test]
    fn supports_returns_true_for_specs_with_the_same_identity_and_compatible_versions() {
        assert!(supports(
            "https://spec.example.com/a/b/mySchema/v1.2/",
            "https://spec.example.com/a/b/mySchema/v1.0/"
        ));
    }

    #[test]
    fn supports_returns_true_for_specs_with_the_same_identity_and_compatible_versions_2() {
        assert!(supports("https://spec.example.com", "https://spec.example.com"));
    }

    #[test]
    fn supports_returns_true_for_specs_with_the_same_identity_and_compatible_versions_3() {
        assert!(supports(
            "https://spec.example.com/mySchema/v0.1?q=v#frag",
            "https://spec.example.com/mySchema/v0.1"
        ));
    }

    #[test]
    fn supports_returns_true_for_specs_with_the_same_identity_and_compatible_versions_4() {
        assert!(supports(
            "https://spec.example.com/v1.100",
            "https://spec.example.com/v1.0"
        ));
    }

    #[test]
    fn supports_returns_true_for_specs_with_the_same_identity_and_compatible_versions_5() {
        assert!(supports("https://spec.example.com/vX", "https://spec.example.com/vX"));
    }

    #[test]
    fn supports_returns_false_for_specs_with_the_same_identity_and_incompatible_versions() {
        assert!(!supports(
            "https://spec.example.com/a/b/mySchema/v1.0/",
            "https://spec.example.com/a/b/mySchema/v1.2/"
        ));
    }

    #[test]
    fn supports_returns_false_for_specs_with_the_same_identity_and_incompatible_versions_2() {
        assert!(!supports(
            "https://spec.example.com/mySchema/v0.1?q=v#frag",
            "https://spec.example.com/mySchema/v0.2"
        ));
    }

    #[test]
    fn supports_returns_false_for_specs_with_the_same_identity_and_incompatible_versions_3() {
        assert!(!supports(
            "https://spec.example.com/v1.0",
            "https://spec.example.com/v1.100"
        ));
    }

    /// The body of "toString returns the normalized url".
    fn to_string_is(url: &str, str: &str) {
        assert_eq!(FederatedLinkUrl::from_url(url).unwrap().to_string(), str);
    }

    #[test]
    fn to_string_returns_the_normalized_url() {
        to_string_is(
            "https://spec.example.com/a/b/mySchema/v1.0/",
            "https://spec.example.com/a/b/mySchema/v1.0",
        );
    }

    #[test]
    fn to_string_returns_the_normalized_url_2() {
        to_string_is("https://spec.example.com", "https://spec.example.com");
    }

    #[test]
    fn to_string_returns_the_normalized_url_3() {
        to_string_is(
            "https://spec.example.com/mySchema/v0.1?q=v#frag",
            "https://spec.example.com/mySchema/v0.1",
        );
    }

    #[test]
    fn to_string_returns_the_normalized_url_4() {
        to_string_is("https://spec.example.com/v1.0", "https://spec.example.com/v1.0");
    }

    #[test]
    fn to_string_returns_the_normalized_url_5() {
        to_string_is("https://spec.example.com/vX", "https://spec.example.com/vX");
    }
}
