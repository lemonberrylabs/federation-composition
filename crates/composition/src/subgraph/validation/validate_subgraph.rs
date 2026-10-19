//! Port of `src/subgraph/validation/validate-subgraph.ts`.
//!
//! Visits: the subgraph document is visited once with
//! `visit_with_type_node_info(visit_in_parallel([state builder visitor, ...federation rules]))`
//! and the `SubgraphValidationContext` as `cx`; then the full type definitions
//! (federation definitions + link spec + subgraph) are visited with
//! `visit_in_parallel(graphql rules)` and the `SimpleValidationContext`. A skipped rule
//! (`__internal.disableValidationRules` contains its `NAME`) is an empty visitor (`{}`).
//! A visitor that throws records the error in the context (`cx.throw`); after the visit,
//! `validate_subgraph` returns it as `Err`.

use std::sync::LazyLock;

use graphql_js::{Definition, Document, GraphQLError, OperationType};
use crate::utils::collections::{IndexMap, IndexSet};

use super::rules::elements::authenticated::AuthenticatedRule;
use super::rules::elements::compose_directive::ComposeDirectiveRules;
use super::rules::elements::context::ContextDirectiveRules;
use super::rules::elements::cost::CostRule;
use super::rules::elements::extends::ExtendsRules;
use super::rules::elements::external::ExternalRules;
use super::rules::elements::field_set::FieldSetRules;
use super::rules::elements::from_context::FromContextDirectiveRules;
use super::rules::elements::inaccessible::InaccessibleRules;
use super::rules::elements::interface_object::InterfaceObjectRules;
use super::rules::elements::key::KeyRules;
use super::rules::elements::list_size::ListSizeRule;
use super::rules::elements::policy::PolicyRule;
use super::rules::elements::provides::ProvidesRules;
use super::rules::elements::r#override::OverrideRules;
use super::rules::elements::requires::RequiresRules;
use super::rules::elements::requires_scopes::RequiresScopesRule;
use super::rules::elements::shareable::ShareableRules;
use super::rules::elements::tag::TagRules;
use super::rules::known_argument_names_on_directives_rule::KnownArgumentNamesOnDirectivesRule;
use super::rules::known_directives_rule::KnownDirectivesRule;
use super::rules::known_federation_directive_rule::KnownFederationDirectivesRule;
use super::rules::known_root_type_rule::KnownRootTypeRule;
use super::rules::known_type_names_rule::KnownTypeNamesRule;
use super::rules::lone_schema_definition_rule::LoneSchemaDefinitionRule;
use super::rules::only_interface_implementation_rule::OnlyInterfaceImplementationRule;
use super::rules::provided_arguments_on_directives_rule::ProvidedArgumentsOnDirectivesRule;
use super::rules::provided_required_arguments_on_directives_rule::ProvidedRequiredArgumentsOnDirectivesRule;
use super::rules::query_root_type_inaccessible_rule::QueryRootTypeInaccessibleRule;
use super::rules::reserved_subgraph_name_rule::ReservedSubgraphNameRule;
use super::rules::root_type_used_rule::RootTypeUsedRule;
use super::rules::unique_argument_definition_names_rule::UniqueArgumentDefinitionNamesRule;
use super::rules::unique_argument_names_rule::UniqueArgumentNamesRule;
use super::rules::unique_directive_names_rule::UniqueDirectiveNamesRule;
use super::rules::unique_directives_per_location_rule::UniqueDirectivesPerLocationRule;
use super::rules::unique_enum_value_names_rule::UniqueEnumValueNamesRule;
use super::rules::unique_field_definition_names_rule::UniqueFieldDefinitionNamesRule;
use super::rules::unique_input_field_names_rule::UniqueInputFieldNamesRule;
use super::rules::unique_operation_types_rule::UniqueOperationTypesRule;
use super::rules::unique_type_names_rule::UniqueTypeNamesRule;
use super::validate_state::validate_subgraph_state;
use super::validation_context::{
    create_simple_validation_context, create_subgraph_validation_context, SimpleRule, SimpleValidationContext,
    SubgraphRef, SubgraphRule, SubgraphValidationContext,
};
use crate::graphql::type_node_info::visit_with_type_node_info;
use crate::specifications::federation::{known_spec_schema, DetectedFederation};
use crate::specifications::link::{parse_link_directive, Link};
use crate::subgraph::state::SubgraphStateBuilder;
use crate::utils::js::{JsError, JsResult};

/// The `subgraph` of `validateSubgraphCore`: `{ name, typeDefs }`.
#[derive(Clone, Copy, Debug)]
pub struct SubgraphCoreInput<'a> {
    pub name: &'a str,
    pub type_defs: &'a Document,
}

/// The `subgraph` of `validateSubgraph`: `{ name, typeDefs, id }`. `type_defs` is mutable
/// because `cleanSubgraphTypeDefsFromSubgraphSpec` edits the document in place (and the
/// caller's graph sees the edit, as in TS).
#[derive(Debug)]
pub struct SubgraphInput<'a> {
    pub name: &'a str,
    pub id: &'a str,
    pub type_defs: &'a mut Document,
}

/// `__internal?: { disableValidationRules?: string[] }`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InternalOptions {
    pub disable_validation_rules: Option<Vec<String>>,
}

/// The result of `extractLinks`/`validateSubgraphCore`: `{ links }` or `{ errors }`.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtractedLinks {
    /// `links` (`None` when `errors` is set).
    pub links: Option<Vec<Link>>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// `assertUniqueSubgraphNames(subgraphs)`: throws `A subgraph named X already exists`.
pub fn assert_unique_subgraph_names<'n>(subgraphs: impl IntoIterator<Item = &'n str>) -> JsResult<()> {
    let mut names: IndexSet<&str> = IndexSet::default();

    for name in subgraphs {
        if names.contains(name) {
            return Err(JsError::error(format!("A subgraph named {name} already exists")));
        }
        names.insert(name);
    }

    Ok(())
}

/// `validateSubgraphCore(subgraph)`: `@link` validation, errors enriched with the subgraph name.
pub fn validate_subgraph_core(subgraph: SubgraphCoreInput<'_>) -> ExtractedLinks {
    let mut extracted_links = extract_links(subgraph);

    if let Some(errors) = extracted_links.errors.take() {
        extracted_links.errors = Some(
            errors
                .into_iter()
                .map(|error| enrich_error_with_subgraph_name(error, subgraph.name))
                .collect(),
        );
    }

    extracted_links
}

/// `validateSubgraph(subgraph, stateBuilder, federation, __internal?)`.
/// `Err` is a throw escaping the TS function (from a visitor or the state builder).
pub fn validate_subgraph(
    subgraph: SubgraphInput<'_>,
    state_builder: &mut SubgraphStateBuilder,
    federation: &DetectedFederation,
    internal: Option<&InternalOptions>,
) -> JsResult<Vec<GraphQLError>> {
    clean_subgraph_type_defs_from_subgraph_spec(subgraph.type_defs);
    let type_defs: &Document = subgraph.type_defs;

    // PERF: parsed once (`LINK_SPEC_DEFINITIONS`) instead of on every call; the TS parses a
    // constant string, so every call sees the same definitions.
    let link_spec_definitions: &'static [Definition] = &LINK_SPEC_DEFINITIONS;

    let rules_to_skip: Vec<String> = internal
        .and_then(|i| i.disable_validation_rules.clone())
        .unwrap_or_default();
    let mut validation_context = create_subgraph_validation_context(
        SubgraphRef {
            name: subgraph.name,
            id: subgraph.id,
            type_defs,
        },
        federation,
        state_builder,
    );

    let mut visitors: Vec<Box<dyn graphql_js::Visitor<'_, SubgraphValidationContext<'_>> + '_>> =
        vec![Box::new(validation_context.state_builder.visitor())];
    let cx = &mut validation_context;
    let skip = &rules_to_skip;
    visitors.push(federation_rule::<ReservedSubgraphNameRule>(skip, cx));
    visitors.push(federation_rule::<KnownFederationDirectivesRule>(skip, cx));
    visitors.push(federation_rule::<FieldSetRules>(skip, cx));
    visitors.push(federation_rule::<InaccessibleRules>(skip, cx));
    visitors.push(federation_rule::<InterfaceObjectRules>(skip, cx));
    visitors.push(federation_rule::<AuthenticatedRule>(skip, cx));
    visitors.push(federation_rule::<PolicyRule>(skip, cx));
    visitors.push(federation_rule::<RequiresScopesRule>(skip, cx));
    visitors.push(federation_rule::<CostRule>(skip, cx));
    visitors.push(federation_rule::<ListSizeRule>(skip, cx));
    visitors.push(federation_rule::<OverrideRules>(skip, cx));
    visitors.push(federation_rule::<ContextDirectiveRules>(skip, cx));
    visitors.push(federation_rule::<FromContextDirectiveRules>(skip, cx));
    visitors.push(federation_rule::<ExtendsRules>(skip, cx));
    visitors.push(federation_rule::<QueryRootTypeInaccessibleRule>(skip, cx));
    visitors.push(federation_rule::<KnownTypeNamesRule>(skip, cx));
    visitors.push(federation_rule::<KnownRootTypeRule>(skip, cx));
    visitors.push(federation_rule::<RootTypeUsedRule>(skip, cx));
    visitors.push(federation_rule::<ShareableRules>(skip, cx));
    visitors.push(federation_rule::<KeyRules>(skip, cx));
    visitors.push(federation_rule::<ProvidesRules>(skip, cx));
    visitors.push(federation_rule::<RequiresRules>(skip, cx));
    visitors.push(federation_rule::<ExternalRules>(skip, cx));
    visitors.push(federation_rule::<TagRules>(skip, cx));
    visitors.push(federation_rule::<ComposeDirectiveRules>(skip, cx));

    let mut visitor = visit_with_type_node_info(graphql_js::visit_in_parallel(visitors));
    graphql_js::visit(type_defs, &mut visitor, &mut validation_context);
    drop(visitor);

    if let Some(error) = validation_context.take_thrown() {
        return Err(error);
    }

    let federation_definition_replacements = validation_context.collect_federation_definition_replacements().clone();

    // Include only link spec definitions that are not already defined in the subgraph
    let link_spec_definitions_to_include: Vec<&'static Definition> = link_spec_definitions
        .iter()
        .filter(|def| match def.name() {
            Some(name) => !validation_context.state_builder.state.types.contains_key(&name.value),
            None => true,
        })
        .collect();

    let available_federation_definitions: Vec<Definition> = validation_context
        .get_available_federation_type_and_directive_definitions()
        .into_iter()
        .filter(|def| {
            // `def.name.value`: every spec definition has a name.
            !def
                .name()
                .is_some_and(|name| federation_definition_replacements.contains(&name.value))
        })
        .collect();
    let link_spec_document: Option<Vec<&'static Definition>> = if validation_context.satisfies_version_range("> v1.0")
        && !validation_context.state_builder.state.specs.link
    {
        // TODO: If Link v1.0 spec is detected in the subgraph (`schema @link(url: ".../link/v1.0")`)
        // We should validate its directives and types
        // just like we do with Federation directives and types.
        if !link_spec_definitions_to_include.is_empty() {
            Some(link_spec_definitions_to_include)
        } else {
            None
        }
    } else {
        None
    };
    // PERF: `concatAST([availableFederationDefinitions, linkSpecDocument, typeDefs])` built
    // directly: the federation definitions are moved instead of cloned twice, and the link spec
    // definitions are cloned once from the shared parsed copy. The subgraph's own definitions
    // are added below, after the last use of `validation_context`.
    let mut full_definitions: Vec<Definition> = Vec::with_capacity(
        available_federation_definitions.len()
            + link_spec_document.as_ref().map_or(0, Vec::len)
            + type_defs.definitions.len(),
    );
    full_definitions.extend(available_federation_definitions);
    full_definitions.extend(link_spec_document.into_iter().flatten().cloned());

    // validate built subgraph state
    let subgraph_state_errors = validate_subgraph_state(&validation_context.state_builder.state, &validation_context);

    // PERF: the TS runs the graphql rules over `fullTypeDefs` here, before marking fields as
    // shareable and collecting the context's errors. The rules only read `fullTypeDefs` and
    // only produce `simpleValidationContext`'s errors, which are appended in the same place
    // below, so they run after those steps instead. That lets the context's borrow of the
    // subgraph document end, and the document's definitions are moved into `fullTypeDefs`
    // for the visit and moved back after it instead of being deep-cloned.
    for field in validation_context.get_fields_to_mark_as_shareable() {
        if validation_context.state_builder.is_interface_object(&field.type_name) {
            continue;
        }

        validation_context
            .state_builder
            .object_type()
            .field()
            .set_shareable(&field.type_name, &field.field_name)?;
    }

    let mut errors = validation_context.collect_reported_errors();
    errors.extend(validation_context.collect_unused_external().into_iter().map(|coordinate| {
        enrich_error_with_subgraph_name(
            GraphQLError::with_code(
                format!(
                    "Field \"{coordinate}\" is marked @external but is not used in any federation directive (@key, @provides, @requires) or to satisfy an interface; the field declaration has no use and should be removed (or the field should not be @external)."
                ),
                "EXTERNAL_UNUSED",
            ),
            subgraph.name,
        )
    }));
    drop(validation_context);

    let subgraph_definitions_start = full_definitions.len();
    full_definitions.append(&mut subgraph.type_defs.definitions);
    let full_type_defs = Document::new(full_definitions);

    let mut simple_validation_context = create_simple_validation_context(&full_type_defs);

    {
        let cx = &mut simple_validation_context;
        let graphql_rules: Vec<Box<dyn graphql_js::Visitor<'_, SimpleValidationContext<'_>> + '_>> = vec![
            graphql_rule::<OnlyInterfaceImplementationRule>(skip, cx),
            graphql_rule::<LoneSchemaDefinitionRule>(skip, cx),
            graphql_rule::<UniqueOperationTypesRule>(skip, cx),
            graphql_rule::<UniqueTypeNamesRule>(skip, cx),
            graphql_rule::<UniqueEnumValueNamesRule>(skip, cx),
            graphql_rule::<UniqueFieldDefinitionNamesRule>(skip, cx),
            graphql_rule::<UniqueArgumentDefinitionNamesRule>(skip, cx),
            graphql_rule::<KnownDirectivesRule>(skip, cx),
            graphql_rule::<UniqueDirectivesPerLocationRule>(skip, cx),
            graphql_rule::<KnownArgumentNamesOnDirectivesRule>(skip, cx),
            graphql_rule::<UniqueArgumentNamesRule>(skip, cx),
            graphql_rule::<UniqueInputFieldNamesRule>(skip, cx),
            graphql_rule::<UniqueDirectiveNamesRule>(skip, cx),
            graphql_rule::<ProvidedRequiredArgumentsOnDirectivesRule>(skip, cx),
            graphql_rule::<ProvidedArgumentsOnDirectivesRule>(skip, cx),
        ];
        let mut visitor = graphql_js::visit_in_parallel(graphql_rules);
        graphql_js::visit(&full_type_defs, &mut visitor, &mut simple_validation_context);
    }

    errors.extend(simple_validation_context.collect_reported_errors());
    drop(simple_validation_context);
    let mut full_definitions = full_type_defs.definitions;
    subgraph.type_defs.definitions = full_definitions.split_off(subgraph_definitions_start);

    errors.extend(subgraph_state_errors);

    Ok(errors
        .into_iter()
        .map(|error| enrich_error_with_subgraph_name(error, subgraph.name))
        .collect())
}

/// The `linkSpecDefinitions` SDL parsed inside `validateSubgraph`.
const LINK_SPEC_DEFINITIONS_SDL: &str = r#"
    enum Purpose {
      EXECUTION
      SECURITY
    }

    directive @link(
      url: String
      as: String
      for: link__Purpose
      import: [link__Import]
    ) repeatable on SCHEMA

    scalar link__Import

    enum link__Purpose {
      """
      `SECURITY` features provide metadata necessary to securely resolve fields.
      """
      SECURITY

      """
      `EXECUTION` features provide metadata necessary for operation execution.
      """
      EXECUTION
    }
  "#;

/// `linkSpecDefinitions`: [`LINK_SPEC_DEFINITIONS_SDL`] parsed.
static LINK_SPEC_DEFINITIONS: LazyLock<Vec<Definition>> = LazyLock::new(|| {
    graphql_js::parse(LINK_SPEC_DEFINITIONS_SDL)
        .expect("the link spec definitions are valid SDL")
        .definitions
});

/// The `{}` visitor used for a disabled rule.
struct EmptyVisitor;

impl<'a, C: ?Sized> graphql_js::Visitor<'a, C> for EmptyVisitor {
    fn kinds(&self) -> graphql_js::VisitKinds {
        graphql_js::VisitKinds::enter(&[])
    }
}

/// Boxes a federation rule, or an empty visitor when the rule is disabled.
/// `federationRules.map((rule) => rulesToSkip.includes(rule.name) ? {} : rule(validationContext))`.
fn federation_rule<'a, R: SubgraphRule<'a>>(
    rules_to_skip: &[String],
    context: &mut SubgraphValidationContext<'a>,
) -> Box<dyn graphql_js::Visitor<'a, SubgraphValidationContext<'a>> + 'a> {
    if rules_to_skip.iter().any(|r| r == R::NAME) {
        return Box::new(EmptyVisitor);
    }
    Box::new(R::new(context))
}

/// Same as [`federation_rule`] for the graphql rules.
fn graphql_rule<'a, R: SimpleRule<'a>>(
    rules_to_skip: &[String],
    context: &mut SimpleValidationContext<'a>,
) -> Box<dyn graphql_js::Visitor<'a, SimpleValidationContext<'a>> + 'a> {
    if rules_to_skip.iter().any(|r| r == R::NAME) {
        return Box::new(EmptyVisitor);
    }
    Box::new(R::new(context))
}

/// `enrichErrorWithSubgraphName(error, subgraphName)`: prefixes `[name] ` and sets
/// `extensions.subgraphName`, unless already set (truthy).
fn enrich_error_with_subgraph_name(mut error: GraphQLError, subgraph_name: &str) -> GraphQLError {
    // Not if it's already enriched
    if error.extensions.get("subgraphName").is_some_and(is_truthy) {
        return error;
    }

    error.message = format!("[{subgraph_name}] {}", error.message);
    error
        .extensions
        .insert("subgraphName".to_string(), serde_json::Value::String(subgraph_name.to_string()));
    error
}

/// JS truthiness of a JSON value.
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => true,
    }
}

// Move somewhere else

/// `availableFeatures`: known versions of non-federation Apollo specs.
fn available_features() -> IndexMap<&'static str, &'static [&'static str]> {
    IndexMap::from_iter([
        ("link", &["v1.0"][..]),
        ("tag", &["v0.1", "v0.2"][..]),
        ("kotlin_labs", &["v0.1", "v0.2"][..]),
        ("join", &["v0.1", "v0.2", "v0.3", "v0.4", "v0.5"][..]),
        ("inaccessible", &["v0.1", "v0.2"][..]),
        ("core", &["v0.1", "v0.2"][..]),
    ])
}

/// The properties a plain object literal inherits from `Object.prototype`.
/// NOTE(port): `availableFeatures[link.name]` reads these for such names: the value is truthy
/// and has no `includes` method, so the TS throws a `TypeError`, which `extractLinks` catches.
const OBJECT_PROTOTYPE_PROPERTIES: [&str; 12] = [
    "constructor",
    "__defineGetter__",
    "__defineSetter__",
    "hasOwnProperty",
    "__lookupGetter__",
    "__lookupSetter__",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toString",
    "valueOf",
    "__proto__",
    "toLocaleString",
];

/// `/^v\d+\.\d+/.test(version)`.
fn is_version_string(version: &str) -> bool {
    let Some(rest) = version.strip_prefix('v') else {
        return false;
    };
    let major_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if major_len == 0 {
        return false;
    }
    let Some(rest) = rest[major_len..].strip_prefix('.') else {
        return false;
    };
    rest.bytes().next().is_some_and(|b| b.is_ascii_digit())
}

/// `${value}` for a `string | null` link field.
fn print_nullable(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("null")
}

fn is_truthy_str(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|v| !v.is_empty())
}

/// `extractLinks(subgraph)`. The `try/catch` around each link turns a throw from
/// `parseLinkDirective`/`createSpecSchema` into an error (`error instanceof GraphQLError ?
/// error : new GraphQLError(String(error))`, i.e. `JsError::into_graphql_error`).
fn extract_links(subgraph: SubgraphCoreInput<'_>) -> ExtractedLinks {
    let schema_nodes: Vec<&Definition> = subgraph
        .type_defs
        .definitions
        .iter()
        .filter(|d| is_schema_definition_or_extension_node(d))
        .collect();

    if schema_nodes.is_empty() {
        return ExtractedLinks {
            links: Some(Vec::new()),
            errors: None,
        };
    }

    let mut link_directives: Vec<&graphql_js::ConstDirective> = Vec::new();

    for schema_node in schema_nodes {
        let directives = match schema_node {
            Definition::SchemaDefinition(d) => &d.directives,
            Definition::SchemaExtension(d) => &d.directives,
            _ => unreachable!("is_schema_definition_or_extension_node"),
        };
        for directive_node in directives {
            if directive_node.name.value == "link" {
                link_directives.push(directive_node);
            }
        }
    }

    // NOTE(port): the TS `if (!linkDirectives) return { links: [] }` tests an array, which is
    // always truthy, so it never returns.

    let mut errors: Vec<GraphQLError> = Vec::new();
    let mut links: Vec<Link> = Vec::new();
    let mut identities: IndexSet<String> = IndexSet::default();
    let mut reported_as_duplicate: IndexSet<String> = IndexSet::default();

    for link_directive in link_directives {
        // The body of the `try` block; `Ok(())` is a `continue` or the end of the iteration.
        let result = (|| -> JsResult<()> {
            let Some(link) = parse_link_directive(link_directive)? else {
                return Ok(());
            };

            if identities.contains(&link.identity) && !reported_as_duplicate.contains(&link.identity) {
                errors.push(GraphQLError::with_code(
                    format!("Duplicate inclusion of feature {}", link.identity),
                    "INVALID_LINK_DIRECTIVE_USAGE",
                ));
                reported_as_duplicate.insert(link.identity.clone());
            }

            identities.insert(link.identity.clone());

            let has_invalid_version = is_truthy_str(&link.version)
                && !is_version_string(link.version.as_deref().unwrap_or_default());
            if has_invalid_version {
                errors.push(GraphQLError::with_code(
                    format!(
                        "Expected a version string (of the form v1.2), got {}",
                        print_nullable(&link.version)
                    ),
                    "INVALID_LINK_IDENTIFIER",
                ));
                return Ok(());
            }

            let is_apollo_specs_link =
                link.identity == "https://specs.apollo.dev" || link.identity.starts_with("https://specs.apollo.dev/");

            if is_apollo_specs_link {
                if !is_truthy_str(&link.name) && !is_truthy_str(&link.version) {
                    errors.push(GraphQLError::with_code(
                        format!(
                            "Invalid @link url '{}': expected '/<feature>/vX.Y' (for example '/federation/v2.9')",
                            link.identity
                        ),
                        "INVALID_LINK_IDENTIFIER",
                    ));
                    return Ok(());
                }

                if !is_truthy_str(&link.version) {
                    // federation must have a version
                    errors.push(GraphQLError::with_code(
                        format!(
                            "Expected a version string (of the form v1.2), got {}",
                            print_nullable(&link.version)
                        ),
                        "INVALID_LINK_IDENTIFIER",
                    ));
                    return Ok(());
                }

                let version = link.version.as_deref().unwrap_or_default();

                if link.name.as_deref() == Some("federation") {
                    // NOTE(port): unreachable (the version was checked above); kept as in TS.
                    if !is_truthy_str(&link.version) {
                        errors.push(GraphQLError::with_code(
                            format!("Missing version in feature url '{}'", link.identity),
                            "INVALID_LINK_IDENTIFIER",
                        ));
                        return Ok(());
                    }

                    // PERF: the shared copy of `createSpecSchema(version)`; only read here.
                    let spec = known_spec_schema(version)?;
                    let available_elements: IndexSet<&str> = spec
                        .directives
                        .iter()
                        .map(|d| d.name.value.as_str())
                        .chain(spec.types.iter().map(|t| t.name().value.as_str()))
                        .collect();

                    // ensure correct imports
                    let mut pushed_error = false;
                    for im in &link.imports {
                        let name = im.name.strip_prefix('@').unwrap_or(&im.name);
                        if !available_elements.contains(name) {
                            pushed_error = true;
                            errors.push(GraphQLError::with_code(
                                format!("Cannot import unknown element \"{}\".", im.name),
                                "INVALID_LINK_DIRECTIVE_USAGE",
                            ));
                        }
                    }

                    if pushed_error {
                        return Ok(());
                    }
                } else if let Some(name) = link.name.as_deref().filter(|n| !n.is_empty()) {
                    let features = available_features();
                    if let Some(versions) = features.get(name) {
                        if !versions.contains(&version) {
                            errors.push(GraphQLError::with_code(
                                format!("Schema uses unknown version {version} of the {name} spec"),
                                "UNKNOWN_LINK_VERSION",
                            ));
                            return Ok(());
                        }
                    } else if OBJECT_PROTOTYPE_PROPERTIES.contains(&name) {
                        return Err(JsError::type_error(
                            "availableFeatures[link.name].includes is not a function",
                        ));
                    }
                }
            }

            links.push(link);
            Ok(())
        })();

        if let Err(error) = result {
            errors.push(error.into_graphql_error());
        }
    }

    if !errors.is_empty() {
        return ExtractedLinks {
            links: None,
            errors: Some(errors),
        };
    }

    ExtractedLinks {
        links: Some(links),
        errors: None,
    }
}

fn is_schema_definition_or_extension_node(node: &graphql_js::Definition) -> bool {
    matches!(
        node,
        graphql_js::Definition::SchemaDefinition(_) | graphql_js::Definition::SchemaExtension(_)
    )
}

/// `cleanSubgraphTypeDefsFromSubgraphSpec(typeDefs)`: removes `_Any`, `_Entity`, `_Service`
/// and the `_service`/`_entities` query fields, in place.
///
/// Removes all types that are specific to the subgraph spec.
/// I'm not sure why there are types like _Any and _Service in the first place,
/// but they should be ignored.
///
/// ```graphql
/// type Query {
///  _service: _Service!
///  _entities(representations: [_Any!]!): [_Entity]!
/// }
/// scalar _Any
/// union _Entity = ...
/// type _Service {
///  sdl: String
/// }
/// ```
fn clean_subgraph_type_defs_from_subgraph_spec(type_defs: &mut Document) {
    let schema_def_operation_types = type_defs.definitions.iter().find_map(|node| {
        let operation_types = match node {
            Definition::SchemaDefinition(d) => &d.operation_types,
            Definition::SchemaExtension(d) => &d.operation_types,
            _ => return None,
        };
        operation_types
            .iter()
            .any(|op| op.operation == OperationType::Query)
            .then_some(operation_types)
    });

    let query_type_name: String = schema_def_operation_types
        .and_then(|ops| ops.iter().find(|op| op.operation == OperationType::Query))
        .map(|op| op.ty.name.value.clone())
        .unwrap_or_else(|| "Query".to_string());

    type_defs.definitions.retain(|def| match def {
        Definition::ScalarTypeDefinition(d) if d.name.value == "_Any" => false,
        Definition::InputObjectTypeDefinition(d) if d.name.value == "_Any" => false,
        Definition::UnionTypeDefinition(d) if d.name.value == "_Entity" => false,
        Definition::ObjectTypeDefinition(d) if d.name.value == "_Service" => false,
        _ => true,
    });

    // `queryTypes`: the kept object type definitions and extensions named like the query root.
    for def in type_defs.definitions.iter_mut() {
        let fields = match def {
            Definition::ObjectTypeDefinition(d) if d.name.value == query_type_name => &mut d.fields,
            Definition::ObjectTypeExtension(d) if d.name.value == query_type_name => &mut d.fields,
            _ => continue,
        };
        fields.retain(|field| !(field.name.value == "_service" || field.name.value == "_entities"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_string_regex() {
        assert!(is_version_string("v1.0"));
        assert!(is_version_string("v12.34-beta"));
        assert!(!is_version_string("v1"));
        assert!(!is_version_string("v1."));
        assert!(!is_version_string("1.0"));
        assert!(!is_version_string("v.1"));
        assert!(!is_version_string("V1.0"));
    }

    #[test]
    fn enrich_error_prefixes_once() {
        let error = enrich_error_with_subgraph_name(GraphQLError::with_code("boom", "X"), "users");
        assert_eq!(error.message, "[users] boom");
        assert_eq!(error.extensions.get("subgraphName"), Some(&serde_json::json!("users")));
        assert_eq!(error.extensions.keys().collect::<Vec<_>>(), vec!["code", "subgraphName"]);
        let again = enrich_error_with_subgraph_name(error, "other");
        assert_eq!(again.message, "[users] boom");

        // An empty (falsy) subgraphName is overwritten.
        let mut error = GraphQLError::new("x");
        error.extensions.insert("subgraphName".into(), serde_json::json!(""));
        assert_eq!(enrich_error_with_subgraph_name(error, "a").message, "[a] x");
    }

    #[test]
    fn unique_subgraph_names() {
        assert!(assert_unique_subgraph_names(["a", "b"]).is_ok());
        let err = assert_unique_subgraph_names(["a", "b", "a"]).unwrap_err();
        assert_eq!(err.to_js_string(), "Error: A subgraph named a already exists");
    }

    #[test]
    fn clean_subgraph_spec_types() {
        let mut doc = graphql_js::parse(
            r#"
            schema { query: RootQuery }
            type RootQuery { _service: _Service! _entities(representations: [_Any!]!): [_Entity]! me: String }
            extend type RootQuery { _entities: Int other: Int }
            type Query { _service: Int }
            scalar _Any
            union _Entity = A
            type _Service { sdl: String }
            extend type _Service { x: Int }
            type A { id: ID }
            "#,
        )
        .unwrap();
        clean_subgraph_type_defs_from_subgraph_spec(&mut doc);
        let printed = graphql_js::print(&doc);
        assert_eq!(
            printed,
            graphql_js::print(
                &graphql_js::parse(
                    r#"
            schema { query: RootQuery }
            type RootQuery { me: String }
            extend type RootQuery { other: Int }
            type Query { _service: Int }
            extend type _Service { x: Int }
            type A { id: ID }
            "#
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn link_spec_definitions_parse() {
        let doc = graphql_js::parse(LINK_SPEC_DEFINITIONS_SDL).unwrap();
        let names: Vec<&str> = doc.definitions.iter().filter_map(|d| d.name()).map(|n| n.value.as_str()).collect();
        assert_eq!(names, vec!["Purpose", "link", "link__Import", "link__Purpose"]);
    }

    #[test]
    fn extract_links_without_schema_node() {
        let doc = graphql_js::parse("type Query { a: Int }").unwrap();
        let result = extract_links(SubgraphCoreInput {
            name: "a",
            type_defs: &doc,
        });
        assert_eq!(result.links, Some(vec![]));
        assert_eq!(result.errors, None);
    }
}
