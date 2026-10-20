//! Port of `__tests__/shared/testkit.ts`, guild side only.
//!
//! The TS harness runs most tests for "apollo" (`@apollo/composition`) and "guild" (this
//! library); only the guild side is ported. The harness also checks that Apollo's gateway
//! accepts every Guild supergraph (`ApolloSupergraph.buildForTests(..).subgraphs().validate()`).
//! Its GraphQL-validity part is kept, using the graphql-js port (see [`try_compose_services`]);
//! the federation-specific part and the `console.warn` about unported error codes are dropped.

use federation_composition_rs::contracts::tag_extraction::SchemaContractFilter;
use federation_composition_rs::compose::{
    compose_services as library_compose_services, CompositionFailure, CompositionResult, CompositionSuccess,
};
use federation_composition_rs::subgraph::validation::validate_subgraph::InternalOptions;
use federation_composition_rs::types::ServiceDefinition;
use federation_composition_rs::utils::js::{parse_float, JsError, JsResult};
use graphql_rs::{assert_valid_schema, build_schema, Document, GraphQLError};

/// `versions`.
pub const VERSIONS: [&str; 10] = ["v2.0", "v2.1", "v2.2", "v2.3", "v2.4", "v2.5", "v2.6", "v2.7", "v2.8", "v2.9"];

/// `federationVersionToJoinSpecVersion`, in `Object.entries` order.
pub const FEDERATION_VERSION_TO_JOIN_SPEC_VERSION: [(&str, &str); 10] = [
    ("v2.0", "v0.3"),
    ("v2.1", "v0.3"),
    ("v2.2", "v0.3"),
    ("v2.3", "v0.3"),
    ("v2.4", "v0.3"),
    ("v2.5", "v0.3"),
    ("v2.6", "v0.3"),
    ("v2.7", "v0.4"),
    ("v2.8", "v0.5"),
    ("v2.9", "v0.5"),
];

/// `federationVersionToJoinSpecVersion[version]`.
pub fn federation_version_to_join_spec_version(version: &str) -> &'static str {
    FEDERATION_VERSION_TO_JOIN_SPEC_VERSION
        .iter()
        .find(|(federation, _)| *federation == version)
        .map(|(_, join)| *join)
        .unwrap_or_else(|| panic!("unknown federation version {version}"))
}

/// `satisfiesVersionRange(range, version)`, `range` being `"< v2.3"`, `">= v2.3"` or `"> v2.3"`.
pub fn satisfies_version_range(range: &str, version: &str) -> bool {
    let (sign, ver) = range.split_once(' ').unwrap_or_else(|| panic!("invalid version range {range:?}"));
    let version_in_range = parse_float(&ver.replacen('v', "", 1));
    let detected_version = parse_float(&version.replacen('v', "", 1));
    match sign {
        "<" => detected_version < version_in_range,
        ">" => detected_version > version_in_range,
        _ => detected_version >= version_in_range,
    }
}

/// `{ name, typeDefs }`.
pub fn service(name: &str, type_defs: Document) -> ServiceDefinition {
    ServiceDefinition { name: name.to_string(), url: None, type_defs }
}

/// `{ name, url, typeDefs }`.
pub fn service_with_url(name: &str, url: &str, type_defs: Document) -> ServiceDefinition {
    ServiceDefinition { name: name.to_string(), url: Some(url.to_string()), type_defs }
}

/// The library's `composeServices(services, __internal)`. A throw (the `Err`) panics with the
/// thrown value, as the TS test would fail with it; use [`try_compose_services`] for tests that
/// expect a throw.
#[track_caller]
pub fn compose_services(services: &[ServiceDefinition], internal: Option<InternalOptions>) -> CompositionResult {
    match try_compose_services(services, internal) {
        Ok(result) => result,
        Err(error) => panic!("composeServices threw: {}", error.to_js_string()),
    }
}

/// `() => composeServices(services, __internal)`, for `toThrow` assertions.
///
/// Like the TS harness, a successful composition's supergraph must build as a valid GraphQL
/// schema, or this throws with the message `ApolloSupergraph.buildForTests` throws for it:
/// `@apollo/federation-internals` wraps the validation errors as
/// `"The schema is not a valid GraphQL schema.. Caused by:\n"` followed by the messages.
pub fn try_compose_services(services: &[ServiceDefinition], internal: Option<InternalOptions>) -> JsResult<CompositionResult> {
    let result = library_compose_services(services, internal.as_ref())?;
    if let CompositionResult::Success(success) = &result {
        let validation = build_schema(success.supergraph_sdl())
            .map_err(|error| error.message)
            .and_then(|schema| assert_valid_schema(&schema));
        if let Err(message) = validation {
            return Err(JsError::error(format!("The schema is not a valid GraphQL schema.. Caused by:\n{message}")));
        }
    }
    Ok(result)
}

/// The guild entry of the harness's `both` list: what `api` is inside `testVersions` and
/// `testImplementations`.
///
/// `api.library` is always `"guild"`, `api.injectIf("guild", s)` is `s`, `api.injectIf("apollo", s)`
/// is `""`, `api.runIf("guild", fn)` runs `fn` and `api.runIf("apollo", fn)` does not: resolve
/// those while porting instead of calling helpers.
#[derive(Clone, Copy, Debug)]
pub struct Api;

impl Api {
    /// `api.composeServices(services)`.
    #[track_caller]
    pub fn compose_services(&self, services: &[ServiceDefinition]) -> CompositionResult {
        compose_services(services, None)
    }

    /// `api.composeServices(services, __internal)`.
    #[track_caller]
    pub fn compose_services_with_internal(&self, services: &[ServiceDefinition], internal: Option<InternalOptions>) -> CompositionResult {
        compose_services(services, internal)
    }

    /// `() => api.composeServices(services)`, for `toThrow` assertions.
    pub fn try_compose_services(&self, services: &[ServiceDefinition]) -> JsResult<CompositionResult> {
        try_compose_services(services, None)
    }
}

/// Property reads on a `CompositionResult` as the TS does them, with `undefined` as `None`:
/// `result.errors`, `result.supergraphSdl`, `result.publicSdl`.
pub trait CompositionResultExt {
    /// `result.errors`.
    fn errors(&self) -> Option<&Vec<GraphQLError>>;
    /// `result.supergraphSdl`.
    fn supergraph_sdl(&self) -> Option<&str>;
    /// `result.publicSdl`. The getter can throw; that panics, as the TS expression would throw.
    fn public_sdl(&self) -> Option<&str>;
}

impl CompositionResultExt for CompositionResult {
    fn errors(&self) -> Option<&Vec<GraphQLError>> {
        match self {
            CompositionResult::Failure(failure) => Some(&failure.errors),
            CompositionResult::Success(_) => None,
        }
    }

    fn supergraph_sdl(&self) -> Option<&str> {
        match self {
            CompositionResult::Success(success) => Some(success.supergraph_sdl()),
            CompositionResult::Failure(_) => None,
        }
    }

    fn public_sdl(&self) -> Option<&str> {
        match self {
            CompositionResult::Success(success) => Some(public_sdl(success)),
            CompositionResult::Failure(_) => None,
        }
    }
}

#[track_caller]
fn public_sdl(success: &CompositionSuccess) -> &str {
    match success.public_sdl() {
        Ok(sdl) => sdl,
        Err(error) => panic!("publicSdl threw: {}", error.to_js_string()),
    }
}

fn describe_errors(errors: &[GraphQLError]) -> String {
    errors
        .iter()
        .map(|e| format!("  [{}] {}", e.code().unwrap_or("UNKNOWN"), e.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// graphql-js `assertValidSchema(buildSchema(sdl))`, which the TS `assertCompositionSuccess`
/// runs on `supergraphSdl` and `publicSdl`. Panics with the message graphql-js throws.
#[track_caller]
pub fn assert_valid_sdl(sdl: &str) {
    let schema = match build_schema(sdl) {
        Ok(schema) => schema,
        Err(error) => panic!("buildSchema threw: {}", error.message),
    };
    if let Err(message) = assert_valid_schema(&schema) {
        panic!("assertValidSchema threw: {message}");
    }
}

/// `expect(() => buildSchema(sdl)).not.toThrow()`.
#[track_caller]
pub fn assert_build_schema_does_not_throw(sdl: &str) {
    if let Err(error) = build_schema(sdl) {
        panic!("expected no throw, buildSchema threw: {}", error.message);
    }
}

/// `assertCompositionSuccess(result)`. Returns the success, which the TS asserts the type of.
#[track_caller]
pub fn assert_composition_success(result: &CompositionResult) -> &CompositionSuccess {
    assert_composition_success_with_message(result, None)
}

/// `assertCompositionSuccess(result, message)`: fails with `message || "Unexpected test failure"`
/// (followed by the errors), then checks that `supergraphSdl` and `publicSdl` are valid schemas.
#[track_caller]
pub fn assert_composition_success_with_message<'r>(result: &'r CompositionResult, message: Option<&str>) -> &'r CompositionSuccess {
    let success = match result {
        CompositionResult::Success(success) => success,
        CompositionResult::Failure(failure) => panic!(
            "{}\n{}",
            message.filter(|m| !m.is_empty()).unwrap_or("Unexpected test failure"),
            describe_errors(&failure.errors)
        ),
    };
    assert_valid_sdl(success.supergraph_sdl());
    // `"publicSdl" in result` is always true for this library's success object.
    assert_valid_sdl(public_sdl(success));
    success
}

/// `assertCompositionFailure(result)`. Returns the failure, which the TS asserts the type of.
#[track_caller]
pub fn assert_composition_failure(result: &CompositionResult) -> &CompositionFailure {
    assert_composition_failure_with_message(result, None)
}

/// `assertCompositionFailure(result, message)`: fails with `message || "Unexpected test failure"`.
#[track_caller]
pub fn assert_composition_failure_with_message<'r>(result: &'r CompositionResult, message: Option<&str>) -> &'r CompositionFailure {
    match result {
        CompositionResult::Failure(failure) => failure,
        CompositionResult::Success(success) => panic!(
            "{}\nsupergraphSdl:\n{}",
            message.filter(|m| !m.is_empty()).unwrap_or("Unexpected test failure"),
            success.supergraph_sdl()
        ),
    }
}

/// `ensureCompositionSuccess(result)`.
#[track_caller]
pub fn ensure_composition_success(result: CompositionResult) -> CompositionSuccess {
    assert_composition_success(&result);
    match result {
        CompositionResult::Success(success) => success,
        CompositionResult::Failure(_) => unreachable!(),
    }
}

/// How an expected error's `message` is matched.
#[derive(Clone, Debug)]
enum MessageMatcher {
    /// `message: "..."`.
    Equals(String),
    /// `message: expect.stringContaining("...")`.
    Containing(String),
}

/// How an expected error's `extensions` are matched.
#[derive(Clone, Debug)]
enum ExtensionsMatcher {
    /// `extensions: expect.objectContaining({ code })`.
    CodeContaining(String),
    /// `extensions: { code }`: the extensions are exactly `{ code }`.
    ExactlyCode(String),
}

/// `expect.objectContaining({ message?, extensions? })` against a `GraphQLError`.
///
/// ```ignore
/// // expect.objectContaining({
/// //   message: expect.stringContaining(`Type of field ...`),
/// //   extensions: expect.objectContaining({ code: "FIELD_TYPE_MISMATCH" }),
/// // })
/// ErrorMatcher::new().message_containing("Type of field ...").code("FIELD_TYPE_MISMATCH")
/// ```
#[derive(Clone, Debug, Default)]
pub struct ErrorMatcher {
    message: Option<MessageMatcher>,
    extensions: Option<ExtensionsMatcher>,
}

impl ErrorMatcher {
    /// `expect.objectContaining({})`.
    pub fn new() -> Self {
        ErrorMatcher::default()
    }

    /// `message: "..."` (equality).
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(MessageMatcher::Equals(message.into()));
        self
    }

    /// `message: expect.stringContaining("...")`.
    pub fn message_containing(mut self, substring: impl Into<String>) -> Self {
        self.message = Some(MessageMatcher::Containing(substring.into()));
        self
    }

    /// `extensions: expect.objectContaining({ code })`.
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.extensions = Some(ExtensionsMatcher::CodeContaining(code.into()));
        self
    }

    /// `extensions: { code }` (the extensions object equals `{ code }`).
    pub fn extensions_exactly_code(mut self, code: impl Into<String>) -> Self {
        self.extensions = Some(ExtensionsMatcher::ExactlyCode(code.into()));
        self
    }

    /// Whether `error` matches.
    pub fn matches(&self, error: &GraphQLError) -> bool {
        let message_matches = match &self.message {
            None => true,
            Some(MessageMatcher::Equals(message)) => error.message == *message,
            Some(MessageMatcher::Containing(substring)) => error.message.contains(substring.as_str()),
        };
        let extensions_match = match &self.extensions {
            None => true,
            Some(ExtensionsMatcher::CodeContaining(code)) => error.code() == Some(code.as_str()),
            Some(ExtensionsMatcher::ExactlyCode(code)) => error.extensions.len() == 1 && error.code() == Some(code.as_str()),
        };
        message_matches && extensions_match
    }
}

/// Where a list of errors comes from: `result.errors` (`Option`, `None` is `undefined`), a
/// slice, or a whole `CompositionResult` (for `expect(result).toEqual(expect.objectContaining({ errors }))`).
pub trait ErrorsSource {
    /// The errors, or `None` when the value has no `errors`.
    fn errors_or_none(&self) -> Option<&[GraphQLError]>;
}

impl ErrorsSource for CompositionResult {
    fn errors_or_none(&self) -> Option<&[GraphQLError]> {
        match self {
            CompositionResult::Failure(failure) => Some(&failure.errors),
            CompositionResult::Success(_) => None,
        }
    }
}

impl ErrorsSource for CompositionFailure {
    fn errors_or_none(&self) -> Option<&[GraphQLError]> {
        Some(&self.errors)
    }
}

impl ErrorsSource for [GraphQLError] {
    fn errors_or_none(&self) -> Option<&[GraphQLError]> {
        Some(self)
    }
}

impl ErrorsSource for Vec<GraphQLError> {
    fn errors_or_none(&self) -> Option<&[GraphQLError]> {
        Some(self)
    }
}

impl<T: ErrorsSource + ?Sized> ErrorsSource for Option<&T> {
    fn errors_or_none(&self) -> Option<&[GraphQLError]> {
        self.and_then(|errors| errors.errors_or_none())
    }
}

impl<T: ErrorsSource + ?Sized> ErrorsSource for &T {
    fn errors_or_none(&self) -> Option<&[GraphQLError]> {
        (**self).errors_or_none()
    }
}

/// `expect(errors).toContainEqual(m)` for each `m` of `expected`, which is also
/// `expect(result).toEqual(expect.objectContaining({ errors: expect.arrayContaining(expected) }))`:
/// every matcher matches at least one error.
#[track_caller]
pub fn assert_errors_contain(errors: impl ErrorsSource, expected: &[ErrorMatcher]) {
    let Some(errors) = errors.errors_or_none() else {
        panic!("expected errors, received no `errors` (composition succeeded)");
    };
    let missing: Vec<&ErrorMatcher> = expected.iter().filter(|m| !errors.iter().any(|e| m.matches(e))).collect();
    if !missing.is_empty() {
        panic!(
            "no error matches:\n{}\nreceived errors:\n{}",
            missing.iter().map(|m| format!("  {m:?}")).collect::<Vec<_>>().join("\n"),
            describe_errors(errors)
        );
    }
}

/// `expect(result).toEqual(expect.objectContaining({ errors: [m1, m2] }))` and
/// `expect(errors).toEqual([m1, m2])`: as many errors as matchers, matched in order.
#[track_caller]
pub fn assert_errors_equal(errors: impl ErrorsSource, expected: &[ErrorMatcher]) {
    let Some(errors) = errors.errors_or_none() else {
        panic!("expected errors, received no `errors` (composition succeeded)");
    };
    let all_match = errors.len() == expected.len() && errors.iter().zip(expected).all(|(e, m)| m.matches(e));
    if !all_match {
        panic!(
            "errors differ\nexpected:\n{}\nreceived errors:\n{}",
            expected.iter().map(|m| format!("  {m:?}")).collect::<Vec<_>>().join("\n"),
            describe_errors(errors)
        );
    }
}

/// `expect(() => f()).toThrow()` (`expected: None`) or `.toThrow(substring)`: `f` threw, and
/// the thrown message contains `substring`.
#[track_caller]
pub fn assert_throws<T: std::fmt::Debug>(result: Result<T, JsError>, expected: Option<&str>) -> JsError {
    match result {
        Ok(value) => panic!("expected a throw, received {value:?}"),
        Err(error) => {
            if let Some(expected) = expected {
                assert!(
                    error.message().contains(expected),
                    "expected the thrown message to contain:\n{expected}\nthrown:\n{}",
                    error.message()
                );
            }
            error
        }
    }
}

/// `expect(() => f()).not.toThrow()`.
#[track_caller]
pub fn assert_does_not_throw<T>(result: Result<T, JsError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected no throw, threw: {}", error.to_js_string()),
    }
}

/// `expect(received).not.toContain(substring)` and `expect(received).not.toMatch(substring)` on a
/// string (or a regex without metacharacters): `received` must be a string, as vitest throws
/// otherwise, and must not contain `substring`.
#[track_caller]
pub fn assert_not_to_match_string(received: Option<&str>, substring: &str) {
    let received = received.unwrap_or_else(|| panic!("expected a string, received undefined"));
    assert!(!received.contains(substring), "expected not to contain {substring:?}\n--- received ---\n{received}\n");
}

/// `expect(result).not.toEqual(expect.objectContaining({ errors: expect.arrayContaining([m]) }))`
/// and `expect(errors).not.toEqual(expect.arrayContaining([m]))`: there are no `errors`
/// (the positive matcher fails on `undefined`, so its negation passes), or none matches `m`.
#[track_caller]
pub fn assert_no_error_matches(errors: impl ErrorsSource, matcher: &ErrorMatcher) {
    if let Some(errors) = errors.errors_or_none() {
        let matching: Vec<&GraphQLError> = errors.iter().filter(|e| matcher.matches(e)).collect();
        assert!(matching.is_empty(), "expected no error to match {matcher:?}, matching: {matching:?}");
    }
}

/// `{ include: new Set(include), exclude: new Set(exclude) }`, a `SchemaContractFilter`.
pub fn contract_filter(include: &[&str], exclude: &[&str]) -> SchemaContractFilter {
    SchemaContractFilter {
        include: include.iter().map(|tag| tag.to_string()).collect(),
        exclude: exclude.iter().map(|tag| tag.to_string()).collect(),
    }
}
