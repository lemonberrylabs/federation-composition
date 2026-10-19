//! Port of `src/compose.ts`.

use std::cell::OnceCell;

use graphql_js::{parse, Definition, Document, GraphQLError};

use crate::graphql::printer::print;
use crate::graphql::transform_supergraph_to_public_schema::transform_supergraph_to_public_schema;
use crate::specifications::cost::CostNames;
use crate::specifications::federation::FederationVersion;
use crate::specifications::link::print_link;
use crate::specifications::{authenticated, context, cost, inaccessible, join, link, policy, requires_scopes, tag};
use crate::subgraph::validation::validate_subgraph::InternalOptions;
use crate::types::ServiceDefinition;
use crate::utils::js::{JsError, JsResult};
use crate::validate::{validate, ValidationResult};

/// `composeServices(services, __internal?)`.
/// `Err` is a throw escaping the TS (`validate` and the parse of the core schema).
pub fn compose_services(services: &[ServiceDefinition], internal: Option<&InternalOptions>) -> JsResult<CompositionResult> {
    let validation_result = match validate(services, internal)? {
        ValidationResult::Failure { errors } => {
            return Ok(CompositionResult::Failure(CompositionFailure { errors }));
        }
        ValidationResult::Success(success) => success,
    };

    let mut root_types_query = false;
    let mut root_types_mutation = false;
    let mut root_types_subscription = false;

    for def in &validation_result.supergraph {
        // NOTE(port): `def.name.value`; every supergraph definition is a named type or
        // directive definition.
        let name = def.as_node_ref().name().map(|n| n.value.as_str());
        if name == Some("Query") {
            root_types_query = true;
        } else if name == Some("Mutation") {
            root_types_mutation = true;
        } else if name == Some("Subscription") {
            root_types_subscription = true;
        }

        if root_types_query && root_types_mutation && root_types_subscription {
            break;
        }
    }

    let used_tag_spec = validation_result.specs.tag;
    let used_context_spec = validation_result.specs.context;
    let used_cost_spec = &validation_result.specs.cost;
    let used_inaccessible_spec = validation_result.specs.inaccessible;
    let used_policy_spec = validation_result.specs.policy;
    let used_requires_scopes_spec = validation_result.specs.requires_scopes;
    let used_authenticated_spec = validation_result.specs.authenticated;

    let mut cost_link_imports = String::new();

    if used_cost_spec.used {
        let mut imports: Vec<String> = Vec::new();
        // `usedCostSpec.names.cost && usedCostSpec.names.cost !== "cost"` (truthy string)
        if let Some(cost_name) = used_cost_spec.names.cost.as_deref() {
            if !cost_name.is_empty() && cost_name != "cost" {
                imports.push(create_link_import_value("@cost", Some(&format!("@{cost_name}"))));
            }
        }

        if let Some(list_size_name) = used_cost_spec.names.list_size.as_deref() {
            if !list_size_name.is_empty() {
                let alias = if list_size_name != "listSize" {
                    Some(format!("@{list_size_name}"))
                } else {
                    None
                };
                imports.push(create_link_import_value("@listSize", alias.as_deref()));
            }
        }

        if !imports.is_empty() {
            cost_link_imports = format!(", import: [{}]", imports.join(", "));
        }
    }

    let federation_version_to_join_version = |version: FederationVersion| -> &'static str {
        match version {
            FederationVersion::V1_0
            | FederationVersion::V2_0
            | FederationVersion::V2_1
            | FederationVersion::V2_2
            | FederationVersion::V2_3
            | FederationVersion::V2_4
            | FederationVersion::V2_5
            | FederationVersion::V2_6 => "v0.3",
            FederationVersion::V2_7 => "v0.4",
            FederationVersion::V2_8 | FederationVersion::V2_9 => "v0.5",
        }
    };

    let core_sdl = format!(
        r#"
    schema
    @link(url: "https://specs.apollo.dev/link/v1.0")
    @link(url: "https://specs.apollo.dev/join/{join_version}", for: EXECUTION)
    {context_link}
    {tag_link}
    {cost_link}
    {inaccessible_link}
    {policy_link}
    {requires_scopes_link}
    {authenticated_link}
    {links}
  {{
    {query}
    {mutation}
    {subscription}
  }}

  {join_sdl}
  {context_sdl}
  {link_sdl}
  {tag_sdl}
  {cost_sdl}
  {inaccessible_sdl}
  {policy_sdl}
  {requires_scopes_sdl}
  {authenticated_sdl}
  "#,
        join_version = federation_version_to_join_version(validation_result.federation_version),
        context_link = if used_context_spec {
            r#"@link(url: "https://specs.apollo.dev/context/v0.1", for: SECURITY)"#
        } else {
            ""
        },
        tag_link = if used_tag_spec {
            r#"@link(url: "https://specs.apollo.dev/tag/v0.3")"#
        } else {
            ""
        },
        cost_link = if used_cost_spec.used {
            format!(r#"@link(url: "https://specs.apollo.dev/cost/v0.1"{cost_link_imports})"#)
        } else {
            String::new()
        },
        inaccessible_link = if used_inaccessible_spec {
            r#"@link(url: "https://specs.apollo.dev/inaccessible/v0.2", for: SECURITY)"#
        } else {
            ""
        },
        policy_link = if used_policy_spec {
            r#"@link(url: "https://specs.apollo.dev/policy/v0.1", for: SECURITY)"#
        } else {
            ""
        },
        requires_scopes_link = if used_requires_scopes_spec {
            r#"@link(url: "https://specs.apollo.dev/requiresScopes/v0.1", for: SECURITY)"#
        } else {
            ""
        },
        authenticated_link = if used_authenticated_spec {
            r#"@link(url: "https://specs.apollo.dev/authenticated/v0.1", for: SECURITY)"#
        } else {
            ""
        },
        links = validation_result
            .links
            .iter()
            .map(print_link)
            .collect::<Vec<_>>()
            .join("\n  "),
        query = if root_types_query { "query: Query" } else { "" },
        mutation = if root_types_mutation { "mutation: Mutation" } else { "" },
        subscription = if root_types_subscription {
            "subscription: Subscription"
        } else {
            ""
        },
        join_sdl = join::sdl(validation_result.federation_version),
        context_sdl = if used_context_spec { context::SDL } else { "" },
        link_sdl = link::SDL,
        tag_sdl = if used_tag_spec { tag::SDL } else { "" },
        cost_sdl = if used_cost_spec.used {
            cost::sdl(&CostNames {
                cost: used_cost_spec.names.cost.clone().unwrap_or_else(|| "cost".to_string()),
                list_size: used_cost_spec
                    .names
                    .list_size
                    .clone()
                    .unwrap_or_else(|| "listSize".to_string()),
            })
        } else {
            String::new()
        },
        inaccessible_sdl = if used_inaccessible_spec { inaccessible::SDL } else { "" },
        policy_sdl = if used_policy_spec { policy::SDL } else { "" },
        requires_scopes_sdl = if used_requires_scopes_spec {
            requires_scopes::SDL
        } else {
            ""
        },
        authenticated_sdl = if used_authenticated_spec {
            authenticated::SDL
        } else {
            ""
        },
    );
    let core = parse(&core_sdl)?;

    let mut definitions: Vec<Definition> = core.definitions;
    definitions.extend(validation_result.supergraph);
    let supergraph_document = Document { definitions };

    Ok(CompositionResult::Success(create_composition_success_read_cache_container(
        supergraph_document,
    )))
}

/// `createLinkImportValue(name, alias)`.
fn create_link_import_value(name: &str, alias: Option<&str>) -> String {
    match alias {
        Some(alias) if !alias.is_empty() => format!("{{ name: \"{name}\", as: \"{alias}\" }}"),
        _ => format!("\"{name}\""),
    }
}

/// `CompositionResult = CompositionFailure | CompositionSuccess`.
#[derive(Debug)]
pub enum CompositionResult {
    Failure(CompositionFailure),
    Success(CompositionSuccess),
}

/// `CompositionFailure`.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionFailure {
    pub errors: Vec<GraphQLError>,
}

/// `CompositionSuccess`, as built by `createCompositionSuccessReadCacheContainer`: the SDL and
/// public schema are computed on first access and cached (`OnceCell`).
#[derive(Debug)]
pub struct CompositionSuccess {
    pub supergraph_document_node: Document,
    supergraph_sdl: OnceCell<String>,
    public_document_node: OnceCell<Document>,
    public_sdl: OnceCell<String>,
}

impl CompositionSuccess {
    /// The `supergraphSdl` getter: `print(supergraphDocumentNode)` (library printer).
    pub fn supergraph_sdl(&self) -> &str {
        self.supergraph_sdl
            .get_or_init(|| print(&self.supergraph_document_node))
    }

    /// The `publicDocumentNode` getter. `transformSupergraphToPublicSchema` can throw.
    /// A throw leaves the cache empty, so the next read tries again (as in TS).
    pub fn public_document_node(&self) -> JsResult<&Document> {
        if let Some(public_document_node) = self.public_document_node.get() {
            return Ok(public_document_node);
        }
        let public_document_node = transform_supergraph_to_public_schema(&self.supergraph_document_node)?;
        Ok(self.public_document_node.get_or_init(|| public_document_node))
    }

    /// The `publicSdl` getter.
    pub fn public_sdl(&self) -> JsResult<&str> {
        if let Some(public_sdl) = self.public_sdl.get() {
            return Ok(public_sdl);
        }
        let public_sdl = print(self.public_document_node()?);
        Ok(self.public_sdl.get_or_init(|| public_sdl))
    }
}

/// `message || "Unexpected test failure"`.
fn unexpected_test_failure(message: Option<&str>) -> JsError {
    match message {
        Some(message) if !message.is_empty() => JsError::error(message),
        _ => JsError::error("Unexpected test failure"),
    }
}

/// `assertCompositionSuccess(compositionResult, message?)`: throws
/// `message || "Unexpected test failure"` on failure.
pub fn assert_composition_success(composition_result: &CompositionResult, message: Option<&str>) -> JsResult<()> {
    if composition_has_errors(composition_result) {
        return Err(unexpected_test_failure(message));
    }
    Ok(())
}

/// `assertCompositionFailure(compositionResult, message?)`.
pub fn assert_composition_failure(composition_result: &CompositionResult, message: Option<&str>) -> JsResult<()> {
    if !composition_has_errors(composition_result) {
        return Err(unexpected_test_failure(message));
    }
    Ok(())
}

/// `compositionHasErrors(compositionResult)`.
pub fn composition_has_errors(composition_result: &CompositionResult) -> bool {
    matches!(composition_result, CompositionResult::Failure(_))
}

/// `createCompositionSuccessReadCacheContainer(supergraphDocumentNode)`: only prints the SDL
/// and builds the public document when requested.
pub fn create_composition_success_read_cache_container(supergraph_document_node: Document) -> CompositionSuccess {
    CompositionSuccess {
        supergraph_document_node,
        supergraph_sdl: OnceCell::new(),
        public_document_node: OnceCell::new(),
        public_sdl: OnceCell::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_import_value() {
        assert_eq!(create_link_import_value("@listSize", None), "\"@listSize\"");
        assert_eq!(create_link_import_value("@listSize", Some("")), "\"@listSize\"");
        assert_eq!(
            create_link_import_value("@cost", Some("@myCost")),
            "{ name: \"@cost\", as: \"@myCost\" }"
        );
    }

    #[test]
    fn assertions() {
        let failure = CompositionResult::Failure(CompositionFailure { errors: vec![] });
        assert!(composition_has_errors(&failure));
        assert_eq!(
            assert_composition_success(&failure, None).unwrap_err().to_js_string(),
            "Error: Unexpected test failure"
        );
        assert_eq!(
            assert_composition_success(&failure, Some("")).unwrap_err().to_js_string(),
            "Error: Unexpected test failure"
        );
        assert_eq!(
            assert_composition_success(&failure, Some("boom")).unwrap_err().to_js_string(),
            "Error: boom"
        );
        assert!(assert_composition_failure(&failure, None).is_ok());

        let success = CompositionResult::Success(create_composition_success_read_cache_container(Document {
            definitions: vec![],
        }));
        assert!(!composition_has_errors(&success));
        assert!(assert_composition_success(&success, None).is_ok());
        assert_eq!(
            assert_composition_failure(&success, Some("x")).unwrap_err().to_js_string(),
            "Error: x"
        );
    }
}
