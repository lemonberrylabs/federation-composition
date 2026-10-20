//! The testkit: the Rust counterpart of `__tests__/shared/{testkit.ts,utils.ts,setup.ts}`
//! and `__tests__/fixtures/*`, guild side only.
//!
//! Every spec module starts with `use crate::shared::*;`. The macros below are in scope
//! everywhere without an import (`#[macro_use] mod shared;` comes first in `main.rs`).
//!
//! ```ignore
//! use crate::shared::*;
//!
//! // testVersions((api, version) => { test("FIELD_TYPE_MISMATCH", () => { ... }) })
//! test_versions! {
//!     fn field_type_mismatch(api, version) {
//!         let result = api.compose_services(&[
//!             service("users", graphql!(r#"
//!                 extend schema @link(url: "https://specs.apollo.dev/federation/${version}", import: ["@key"])
//!                 type Query { users: [User!]! }
//!             "#, version)),
//!         ]);
//!         assert_composition_success(&result);
//!     }
//!
//!     // if (satisfiesVersionRange(">= v2.7", version)) { test(...) }
//!     fn label_is_accepted(api, version) if ">= v2.7" { /* ... */ }
//!
//!     // if (version === "v2.0") { test(...) }
//!     fn only_on_v2_0(api, version) if ["v2.0"] { /* ... */ }
//!
//!     #[ignore = "skipped in TypeScript"]
//!     fn skipped(api, version) { /* ported body */ }
//! }
//!
//! // testImplementations((api) => { test("...", () => { ... }) })
//! test_implementations! {
//!     fn composition_of_basic_object_types(api) { /* ... */ }
//! }
//! ```
//!
//! `test_versions!` turns each `fn` into a module of that name holding one `#[test]` per
//! federation version (`field_type_mismatch::v2_0` ... `field_type_mismatch::v2_9`), with `api`
//! bound to [`Api`] and `version` to the version string (`"v2.0"`). The module does
//! `use super::*`, so the body sees everything the spec module sees. `test_implementations!`
//! makes one `#[test] fn` per `fn`.

// The testkit ports all of `__tests__/shared` and `__tests__/fixtures`; a few of those helpers
// (`versions`, `SortSdlSerializer`, `notToEqualGraphQL`, ...) have no caller in the specs.
#![allow(dead_code, unused_imports)]

mod fixtures;
mod setup;
mod snapshot;
mod testkit;
mod utils;

pub use fixtures::*;
pub use setup::*;
pub use snapshot::*;
pub use testkit::*;
pub use utils::*;

pub use federation_composition::__internal::graphql::sort_sdl::sort_sdl;
pub use federation_composition::__internal::subgraph::validation::validate_subgraph::InternalOptions;
pub use federation_composition::__internal::utils::js::{JsError, JsResult};
pub use federation_composition::{CompositionFailure, CompositionResult, CompositionSuccess, ServiceDefinition};
pub use graphql_core::{parse, print, Document, GraphQLError};

/// A JS template literal: replaces each `${name}` with the value of `name`.
/// `tpl!(s, version)` binds `version` to the variable of that name,
/// `tpl!(s, join = federation_version_to_join_spec_version(version))` to an expression.
/// Panics if a `${` is left over (a missing argument). Returns a `String`.
macro_rules! tpl {
    (@value $name:ident) => {
        $name
    };
    (@value $name:ident = $value:expr) => {
        $value
    };
    ($s:expr $(,)?) => {
        $crate::shared::interpolate($s, &[])
    };
    ($s:expr, $($name:ident $(= $value:expr)?),+ $(,)?) => {
        $crate::shared::interpolate(
            $s,
            &[$((stringify!($name), &tpl!(@value $name $(= $value)?) as &dyn ::std::fmt::Display)),+],
        )
    };
}

/// The `graphql` template tag: `graphql!(r#"..."#, version)` is
/// `graphql(&tpl!(r#"..."#, version))`, a `Document` parsed without locations.
macro_rules! graphql {
    ($($args:tt)*) => {
        $crate::shared::graphql(&tpl!($($args)*))
    };
}

/// The `normalizeErrorMessage` template tag: `normalize_error_message(&tpl!(...))`.
macro_rules! normalize_error_message {
    ($($args:tt)*) => {
        $crate::shared::normalize_error_message(&tpl!($($args)*))
    };
}

/// `testVersions((api, version) => { test(...) ... })`. See the module documentation.
macro_rules! test_versions {
    () => {};
    ($(#[$meta:meta])* fn $name:ident($api:ident, $version:ident) if [$($v:tt),* $(,)?] $body:block $($rest:tt)*) => {
        mod $name {
            #[allow(unused_imports)]
            use super::*;
            __version_tests!([$(#[$meta])*] $api $version $body; $($v)*);
        }
        test_versions!($($rest)*);
    };
    ($(#[$meta:meta])* fn $name:ident($api:ident, $version:ident) if $range:tt $body:block $($rest:tt)*) => {
        mod $name {
            #[allow(unused_imports)]
            use super::*;
            __versions_in_range!($range [$(#[$meta])*] $api $version $body);
        }
        test_versions!($($rest)*);
    };
    ($(#[$meta:meta])* fn $name:ident($api:ident, $version:ident) $body:block $($rest:tt)*) => {
        mod $name {
            #[allow(unused_imports)]
            use super::*;
            __version_tests!(
                [$(#[$meta])*] $api $version $body;
                "v2.0" "v2.1" "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"
            );
        }
        test_versions!($($rest)*);
    };
}

/// `testImplementations((api) => { test(...) ... })`: one `#[test]` per `fn`.
macro_rules! test_implementations {
    () => {};
    ($(#[$meta:meta])* fn $name:ident($api:ident) $body:block $($rest:tt)*) => {
        #[test]
        $(#[$meta])*
        fn $name() {
            let $api = $crate::shared::Api;
            let _ = &$api;
            $body
        }
        test_implementations!($($rest)*);
    };
}

#[doc(hidden)]
macro_rules! __version_tests {
    ($attrs:tt $api:ident $version:ident $body:block;) => {};
    ($attrs:tt $api:ident $version:ident $body:block; $v:tt $($rest:tt)*) => {
        __version_test!($v $attrs $api $version $body);
        __version_tests!($attrs $api $version $body; $($rest)*);
    };
}

#[doc(hidden)]
macro_rules! __version_test_fn {
    ($fn_name:ident $v:tt [$($attr:tt)*] $api:ident $version:ident $body:block) => {
        #[test]
        $($attr)*
        fn $fn_name() {
            let $api = $crate::shared::Api;
            let $version: &'static str = $v;
            let _ = (&$api, $version);
            $body
        }
    };
}

#[doc(hidden)]
macro_rules! __version_test {
    ("v2.0" $($args:tt)*) => { __version_test_fn!(v2_0 "v2.0" $($args)*); };
    ("v2.1" $($args:tt)*) => { __version_test_fn!(v2_1 "v2.1" $($args)*); };
    ("v2.2" $($args:tt)*) => { __version_test_fn!(v2_2 "v2.2" $($args)*); };
    ("v2.3" $($args:tt)*) => { __version_test_fn!(v2_3 "v2.3" $($args)*); };
    ("v2.4" $($args:tt)*) => { __version_test_fn!(v2_4 "v2.4" $($args)*); };
    ("v2.5" $($args:tt)*) => { __version_test_fn!(v2_5 "v2.5" $($args)*); };
    ("v2.6" $($args:tt)*) => { __version_test_fn!(v2_6 "v2.6" $($args)*); };
    ("v2.7" $($args:tt)*) => { __version_test_fn!(v2_7 "v2.7" $($args)*); };
    ("v2.8" $($args:tt)*) => { __version_test_fn!(v2_8 "v2.8" $($args)*); };
    ("v2.9" $($args:tt)*) => { __version_test_fn!(v2_9 "v2.9" $($args)*); };
}

/// The versions `v` of [`VERSIONS`] for which `satisfies_version_range(range, v)` holds,
/// resolved when the macro expands (the TS registers the test only for those versions).
#[doc(hidden)]
macro_rules! __versions_in_range {
    ("< v2.0" $($args:tt)*) => { __version_tests!($($args)*; ); };
    ("< v2.1" $($args:tt)*) => { __version_tests!($($args)*; "v2.0"); };
    ("< v2.2" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1"); };
    ("< v2.3" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2"); };
    ("< v2.4" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2" "v2.3"); };
    ("< v2.5" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2" "v2.3" "v2.4"); };
    ("< v2.6" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2" "v2.3" "v2.4" "v2.5"); };
    ("< v2.7" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2" "v2.3" "v2.4" "v2.5" "v2.6"); };
    ("< v2.8" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7"); };
    ("< v2.9" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8"); };
    (">= v2.0" $($args:tt)*) => { __version_tests!($($args)*; "v2.0" "v2.1" "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    (">= v2.1" $($args:tt)*) => { __version_tests!($($args)*; "v2.1" "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    (">= v2.2" $($args:tt)*) => { __version_tests!($($args)*; "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    (">= v2.3" $($args:tt)*) => { __version_tests!($($args)*; "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    (">= v2.4" $($args:tt)*) => { __version_tests!($($args)*; "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    (">= v2.5" $($args:tt)*) => { __version_tests!($($args)*; "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    (">= v2.6" $($args:tt)*) => { __version_tests!($($args)*; "v2.6" "v2.7" "v2.8" "v2.9"); };
    (">= v2.7" $($args:tt)*) => { __version_tests!($($args)*; "v2.7" "v2.8" "v2.9"); };
    (">= v2.8" $($args:tt)*) => { __version_tests!($($args)*; "v2.8" "v2.9"); };
    (">= v2.9" $($args:tt)*) => { __version_tests!($($args)*; "v2.9"); };
    ("> v2.0" $($args:tt)*) => { __version_tests!($($args)*; "v2.1" "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    ("> v2.1" $($args:tt)*) => { __version_tests!($($args)*; "v2.2" "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    ("> v2.2" $($args:tt)*) => { __version_tests!($($args)*; "v2.3" "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    ("> v2.3" $($args:tt)*) => { __version_tests!($($args)*; "v2.4" "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    ("> v2.4" $($args:tt)*) => { __version_tests!($($args)*; "v2.5" "v2.6" "v2.7" "v2.8" "v2.9"); };
    ("> v2.5" $($args:tt)*) => { __version_tests!($($args)*; "v2.6" "v2.7" "v2.8" "v2.9"); };
    ("> v2.6" $($args:tt)*) => { __version_tests!($($args)*; "v2.7" "v2.8" "v2.9"); };
    ("> v2.7" $($args:tt)*) => { __version_tests!($($args)*; "v2.8" "v2.9"); };
    ("> v2.8" $($args:tt)*) => { __version_tests!($($args)*; "v2.9"); };
    ("> v2.9" $($args:tt)*) => { __version_tests!($($args)*; ); };
}
