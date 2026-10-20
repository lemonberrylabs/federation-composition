//! Port of `__tests__/subgraph-validation-and-compositon-root-query.spec.ts`.

use crate::shared::*;

use federation_composition_rs::__internal::validate::validate;
use federation_composition_rs::ValidationResult;
use graphql_rs::Definition;

fn service_a() -> ServiceDefinition {
    service(
        "serviceA",
        parse(
            r#"#graphql
        schema {
          query: CustomQueryName
        }
        type CustomQueryName {
          helloFromServiceA: String
        }
      "#,
        )
        .unwrap(),
    )
}

fn service_b() -> ServiceDefinition {
    service(
        "serviceB",
        parse(
            r#"#graphql
        type Query {
          helloFromServiceB: String
        }
      "#,
        )
        .unwrap(),
    )
}

fn service_c() -> ServiceDefinition {
    service(
        "serviceC",
        parse(
            r#"#graphql
        schema {
          query: RootQuery
        }
        type RootQuery {
          helloFromServiceC: String
        }
      "#,
        )
        .unwrap(),
    )
}

/// `const subgraphServices = [serviceA, serviceB, serviceC]`.
fn subgraph_services() -> Vec<ServiceDefinition> {
    vec![service_a(), service_b(), service_c()]
}

mod test_to_validate_custom_root_query_is_treated_as_query {
    use super::*;

    #[test]
    fn should_treat_root_query_and_custom_query_name_as_query_when_validating_subgraphs() {
        let validation_result = validate(&subgraph_services(), None).unwrap();

        let ValidationResult::Success(validation_result) = validation_result else {
            panic!("Validation failed");
        };

        let supergraph = validation_result.supergraph();

        let has_custom_root_query_as_object_type = supergraph.iter().any(|def| {
            matches!(def, Definition::ObjectTypeDefinition(def)
                if def.name.value == "RootQuery" || def.name.value == "CustomQueryName")
        });

        assert!(!has_custom_root_query_as_object_type);
    }

    #[test]
    fn should_treat_root_query_and_custom_query_name_as_query_when_merging_subgraphs_into_supergraph() {
        let composition_result = compose_services(&subgraph_services(), None);

        // Ensure composition was successful
        // (the library's `assertCompositionSuccess` from `src/compose.ts`, not the testkit's)
        if let Err(error) =
            federation_composition_rs::__internal::compose::assert_composition_success(&composition_result, None)
        {
            panic!("{}", error.to_js_string());
        }

        let expected_query_fields = ["helloFromServiceA", "helloFromServiceB", "helloFromServiceC"];

        // Check the composed supergraph schema to see if both RootQuery and CustomQueryName were treated as Query
        let supergraph_sdl = composition_result.supergraph_sdl().expect("supergraphSdl");
        for expected_field in expected_query_fields {
            assert!(
                supergraph_sdl.contains(expected_field),
                "expected supergraphSdl to include {expected_field}"
            );
        }
    }
}
