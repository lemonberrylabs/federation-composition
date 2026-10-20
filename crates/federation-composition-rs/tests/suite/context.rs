//! Port of `__tests__/context.spec.ts`.
//!
//! The TS file installs a snapshot serializer for `specs.apollo.dev` strings, but none of its
//! tests uses a snapshot, so it has no counterpart here.

use crate::shared::*;

/// `federationSchema(version, document, extraImports)`: `document`'s definitions after an
/// `extend schema @link` of federation `version` that imports the context directives, the
/// other directives the tests use, and `extra_imports`.
fn federation_schema(version: &str, document: Document, extra_imports: &[&str]) -> Document {
    let federation_imports: Vec<&str> = [
        "@key",
        "@shareable",
        "@external",
        "@context",
        "@fromContext",
        "@authenticated",
        "@interfaceObject",
    ]
    .into_iter()
    .chain(extra_imports.iter().copied())
    .collect();
    let linked_schema = graphql!(
        r#"
    extend schema
      @link(
        url: "https://specs.apollo.dev/federation/${version}"
        import: [
          ${imports}
        ]
      )
  "#,
        version,
        imports = federation_imports
            .iter()
            .map(|directive| format!("\"{directive}\""))
            .collect::<Vec<_>>()
            .join("\n          "),
    );

    let mut definitions = linked_schema.definitions;
    definitions.extend(document.definitions);
    Document::new(definitions)
}

/// `expectContextSpecDefinitions(supergraphSdl)`.
#[track_caller]
fn expect_context_spec_definitions(supergraph_sdl: Option<&str>) {
    to_contain_graphql(
        supergraph_sdl,
        graphql!(r#"
        schema
          @link(for: EXECUTION, url: "https://specs.apollo.dev/join/v0.5")
          @link(for: SECURITY, url: "https://specs.apollo.dev/context/v0.1")
          @link(url: "https://specs.apollo.dev/link/v1.0") {
          query: Query
        }
      "#),
    );
    to_contain_graphql(
        supergraph_sdl,
        graphql!(r#"
        directive @context(
          name: String!
        ) repeatable on INTERFACE | OBJECT | UNION
      "#),
    );
    to_contain_graphql(
        supergraph_sdl,
        graphql!(r#"
        scalar context__ContextFieldValue
      "#),
    );
    to_contain_graphql(
        supergraph_sdl,
        graphql!(r#"
        directive @context__fromContext(
          field: context__ContextFieldValue
        ) on ARGUMENT_DEFINITION
      "#),
    );
    to_contain_graphql(
        supergraph_sdl,
        graphql!(r#"
        input join__ContextArgument {
          name: String!
          type: String!
          context: String!
          selection: join__FieldValue!
        }
      "#),
    );
}

/// `contextNotFound(graphName, typeName, fieldName)` of the "context reachability" tests.
fn context_not_found(graph_name: &str, type_name: &str, field_name: &str) -> String {
    format!(r#"- from subgraph "{graph_name}": could not find a match for required context for field "{type_name}.{field_name}"."#)
}

/// `unsatisfiable(queryString, ...reasons)` of the "context reachability" tests.
fn unsatisfiable(query_string: &str, reasons: &[&str]) -> ErrorMatcher {
    let query = normalize_error_message(query_string);
    let message = [
        "The following supergraph API query:",
        query.as_str(),
        "cannot be satisfied by the subgraphs because:",
    ]
    .into_iter()
    .chain(reasons.iter().copied())
    .collect::<Vec<_>>()
    .join("\n");
    ErrorMatcher::new().message(message).code("SATISFIABILITY_ERROR")
}

test_versions! {
    // rejects the context directives before federation 2.8
    fn rejects_the_context_directives_before_federation_2_8(api, version) if "< v2.8" {
        let result = api.compose_services(&[
            service(
                "orders",
                federation_schema(
                    version,
                    graphql!(r#"
              type Customer @key(fields: "id") @context(name: "customerCtx") {
                id: ID!
                segment: String!
              }

              type Query {
                customer(id: ID!): Customer
              }
            "#),
                    &[],
                ),
            ),
        ]);

        assert_composition_failure(&result);
        assert_errors_contain(
            result.errors(),
            &[ErrorMatcher::new()
                .message(r#"[orders] Cannot import unknown element "@context"."#)
                .code("INVALID_LINK_DIRECTIVE_USAGE")],
        );
    }
}

// @context and @fromContext location
mod context_and_from_context_location {
    use super::*;

    // the context name
    mod the_context_name {
        use super::*;

        test_versions! {
            // rejects an empty context name
            fn rejects_an_empty_context_name(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "accounts",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Account @key(fields: "id") @context(name: "") {
                  id: ID!
                }

                type Query {
                  account(id: ID!): Account
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[accounts] Context name "" is invalid. It should have only alphanumeric characters."#)
                        .code("CONTEXT_NAME_INVALID")],
                );
            }

            // rejects a context name that has an underscore
            fn rejects_a_context_name_that_has_an_underscore(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "accounts",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Account @key(fields: "id") @context(name: "bad_name") {
                  id: ID!
                }

                type Query {
                  account(id: ID!): Account
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[accounts] Context name "bad_name" is invalid. It should have only alphanumeric characters."#)
                        .code("CONTEXT_NAME_INVALID")],
                );
            }
        }
    }

    // argument
    mod argument {
        use super::*;

        test_versions! {
            // rejects @fromContext on argument that has a default value
            fn rejects_from_context_on_argument_that_has_a_default_value(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "accounts",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Account @key(fields: "id") @context(name: "accountCtx") {
                  id: ID!
                  locale: String!
                }

                type Formatter @key(fields: "id") {
                  id: ID!
                  format(
                    locale: String = "en-US"
                      @fromContext(field: "$accountCtx { locale }")
                  ): Float!
                }

                type Query {
                  account(id: ID!): Account
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[accounts] @fromContext arguments may not have a default value: "Formatter.format(locale:)"."#)
                        .code("CONTEXT_NOT_SET")],
                );
            }

            // rejects @fromContext on the argument of a directive
            fn rejects_from_context_on_the_argument_of_a_directive(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "accounts",
                        federation_schema(
                            version,
                            graphql!(r#"
                directive @tenant(
                  locale: String @fromContext(field: "$accountCtx { locale }")
                ) on FIELD_DEFINITION

                type Account @key(fields: "id") @context(name: "accountCtx") {
                  id: ID!
                  locale: String!
                }

                type Query {
                  account(id: ID!): Account
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[accounts] @fromContext argument cannot be used on a directive definition "@tenant(locale:)"."#)
                        .code("CONTEXT_NOT_SET")],
                );
            }

            // rejects @fromContext if the parent type is abstract
            fn rejects_from_context_if_the_parent_type_is_abstract(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface MeteredDevice
                  @key(fields: "id")
                  @context(name: "meterCtx") {
                  id: ID!
                  voltage: Float!
                  trip(
                    v: Float @fromContext(field: "$meterCtx { voltage }")
                  ): Boolean!
                }

                type Sensor implements MeteredDevice @key(fields: "id") {
                  id: ID!
                  voltage: Float!
                  trip(v: Float): Boolean!
                }

                type Query {
                  device: MeteredDevice
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] @fromContext argument cannot be used on a field that exists on an abstract type "MeteredDevice.trip(v:)"."#)
                        .code("CONTEXT_NOT_SET")],
                );
            }

            // rejects @fromContext on a field that implements an interface field
            fn rejects_from_context_on_a_field_that_implements_an_interface_field(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "accounts",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Renderable {
                  id: ID!
                  render(locale: String): Float!
                }

                type Account @key(fields: "id") @context(name: "accountCtx") {
                  id: ID!
                  locale: String!
                }

                type Formatter implements Renderable @key(fields: "id") {
                  id: ID!
                  render(
                    locale: String @fromContext(field: "$accountCtx { locale }")
                  ): Float!
                }

                type Query {
                  account(id: ID!): Account
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[accounts] @fromContext argument cannot be used on a field implementing an interface field "Renderable.render"."#)
                        .code("CONTEXT_NOT_SET")],
                );
            }

            // rejects @fromContext on an implementation if the interface field has no argument
            fn rejects_from_context_on_an_implementation_if_the_interface_field_has_no_argument(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "accounts",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Renderable {
                  id: ID!
                  render: Boolean!
                }

                type Account @key(fields: "id") @context(name: "accountCtx") {
                  id: ID!
                  locale: String!
                  formatter: Formatter!
                }

                type Formatter implements Renderable @key(fields: "id") {
                  id: ID!
                  render(
                    locale: String @fromContext(field: "$accountCtx { locale }")
                  ): Boolean!
                }

                type Query {
                  account(id: ID!): Account
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[accounts] @fromContext argument cannot be used on a field implementing an interface field "Renderable.render"."#)
                        .code("CONTEXT_NOT_SET")],
                );
            }
        }
    }
}

// invalid references and unsupported selection syntax
mod invalid_references_and_unsupported_selection_syntax {
    use super::*;

    test_versions! {
        // rejects a selected field that the provider type does not have
        fn rejects_a_selected_field_that_the_provider_type_does_not_have(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "invoices",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                id: ID!
                reference: String!
              }

              type InvoiceAuditor @key(fields: "id") {
                id: ID!
                audit(
                  reference: String
                    @fromContext(field: "$invoiceCtx { amount }")
                ): Float!
              }

              type Query {
                invoice(id: ID!): Invoice
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[invoices] Context "invoiceCtx" is used in "InvoiceAuditor.audit(reference:)" but the selection is invalid for type Invoice. Error: Cannot query field "amount" on type "Invoice"."#)
                    .code("CONTEXT_INVALID_SELECTION")],
            );
        }
    }

    // unsupported selection syntax
    mod unsupported_selection_syntax {
        use super::*;

        test_versions! {
            // rejects two sibling fields inside @fromContext
            fn rejects_two_sibling_fields_inside_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                  type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                    id: ID!
                    reference: String!
                    amount: Float!
                  }

                  type InvoiceAuditor @key(fields: "id") {
                    id: ID!
                    audit(reference: String @fromContext(field: "$invoiceCtx { reference amount }")): Boolean!
                  }

                  type Query {
                    invoice(id: ID!): Invoice
                  }
                "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] Context "invoiceCtx" is used in "InvoiceAuditor.audit(reference:)" but the selection is invalid: multiple selections are made"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a field alias inside @fromContext
            fn rejects_a_field_alias_inside_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                  type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                    id: ID!
                    reference: String!
                    amount: Float!
                  }

                  type InvoiceAuditor @key(fields: "id") {
                    id: ID!
                    audit(reference: String @fromContext(field: "$invoiceCtx { value: reference }")): Boolean!
                  }

                  type Query {
                    invoice(id: ID!): Invoice
                  }
                "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] Context "invoiceCtx" is used in "InvoiceAuditor.audit(reference:)" but the selection is invalid: aliases are not allowed in the selection"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a query directive inside @fromContext
            fn rejects_a_query_directive_inside_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                  type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                    id: ID!
                    reference: String!
                    amount: Float!
                  }

                  type InvoiceAuditor @key(fields: "id") {
                    id: ID!
                    audit(reference: String @fromContext(field: "$invoiceCtx { reference @skip(if: true) }")): Boolean!
                  }

                  type Query {
                    invoice(id: ID!): Invoice
                  }
                "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] Context "invoiceCtx" is used in "InvoiceAuditor.audit(reference:)" but the selection is invalid: directives are not allowed in the selection"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects unsupported syntax before reporting multiple selections inside @fromContext
            fn rejects_unsupported_syntax_before_reporting_multiple_selections_inside_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                  type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                    id: ID!
                    reference: String!
                    amount: Float!
                  }

                  type InvoiceAuditor @key(fields: "id") {
                    id: ID!
                    audit(reference: String @fromContext(field: "$invoiceCtx { reference amount @skip(if: true) }")): Boolean!
                  }

                  type Query {
                    invoice(id: ID!): Invoice
                  }
                "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] Context "invoiceCtx" is used in "InvoiceAuditor.audit(reference:)" but the selection is invalid: directives are not allowed in the selection"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects fragment spreads inside a @fromContext
            fn rejects_fragment_spreads_inside_a_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                  id: ID!
                  reference: String!
                }

                type InvoiceAuditor @key(fields: "id") {
                  id: ID!
                  audit(
                    reference: String
                      @fromContext(field: "$invoiceCtx { ...InvoiceFragment }")
                  ): Int!
                }

                type Query {
                  invoice(id: ID!): Invoice
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] Context "invoiceCtx" is used in "InvoiceAuditor.audit(reference:)" but the selection is invalid: fragment spread is not allowed"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects @fromContext that mixes fields and inline fragments
            fn rejects_from_context_that_mixes_fields_and_inline_fragments(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeviceSource = Sensor

                type Sensor @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    model: String
                      @fromContext(
                        field: "$deviceCtx { ... on Sensor { model } model }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeviceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(model:)" but the selection is invalid: multiple fields could be selected"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a field with an inline fragment in @fromContext
            fn rejects_a_field_with_an_inline_fragment_in_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeviceSource = Sensor

                type Sensor @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    model: String
                      @fromContext(
                        field: "$deviceCtx { model ... on Sensor { model } }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeviceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(model:)" but the selection is invalid: multiple selections are made"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }
        }
    }

    // the context reference
    mod the_context_reference {
        use super::*;

        test_versions! {
            // rejects a @fromContext that starts with a selection
            fn rejects_a_from_context_that_starts_with_a_selection(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                  id: ID!
                  reference: String!
                }

                type InvoiceAuditor @key(fields: "id") {
                  id: ID!
                  audit(
                    reference: String @fromContext(field: "{ reference }")
                  ): Int!
                }

                type Query {
                  invoice(id: ID!): Invoice
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] @fromContext argument does not reference a context "{ reference }"."#)
                        .code("NO_CONTEXT_IN_SELECTION")],
                );
            }

            // rejects @fromContext when there's no @context
            fn rejects_from_context_when_there_s_no_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Invoice @key(fields: "id") {
                  id: ID!
                  reference: String!
                  audit(
                    reference: String
                      @fromContext(field: "$invoiceCtx { reference }")
                  ): Float!
                }

                type Query {
                  invoice(id: ID!): Invoice
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] Context "invoiceCtx" is used at location "Invoice.audit(reference:)" but is never set."#)
                        .code("CONTEXT_NOT_SET")],
                );
            }

            // rejects a @fromContext that is only a field name
            fn rejects_a_from_context_that_is_only_a_field_name(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "invoices",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Invoice @key(fields: "id") @context(name: "invoiceCtx") {
                  id: ID!
                  reference: String!
                }

                type InvoiceAuditor @key(fields: "id") {
                  id: ID!
                  audit(
                    reference: String @fromContext(field: "reference")
                  ): Int!
                }

                type Query {
                  invoice(id: ID!): Invoice
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[invoices] @fromContext argument does not reference a context "reference"."#)
                        .code("NO_CONTEXT_IN_SELECTION")],
                );
            }
        }
    }

    // type conditions
    mod type_conditions {
        use super::*;

        test_versions! {
            // rejects interface type conditions in inline fragments
            fn rejects_interface_type_conditions_in_inline_fragments(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Meter {
                  id: ID!
                  voltage: Float!
                }

                type Sensor implements Meter
                  @key(fields: "id")
                  @context(name: "deviceCtx") {
                  id: ID!
                  voltage: Float!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    voltage: Float
                      @fromContext(
                        field: "$deviceCtx { ... on Meter { voltage } }"
                      )
                  ): Int!
                }

                type Query {
                  sensor: Sensor
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(voltage:)" but the selection is invalid: no type condition matches the location "Sensor""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a type condition that the selection does not use
            fn rejects_a_type_condition_that_the_selection_does_not_use(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeviceSource = Sensor | Controller

                type Camera {
                  id: ID!
                  model: String!
                }

                type Sensor @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Controller @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    model: String
                      @fromContext(
                        field: "$deviceCtx { ... on Sensor { model } ... on Controller { model } ... on Camera { model } }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeviceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(model:)" but the selection is invalid: type condition "Camera" is never used."#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects an inline fragment branch that is not a runtime type
            fn rejects_an_inline_fragment_branch_that_is_not_a_runtime_type(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeviceSource = Sensor | Controller

                type Camera {
                  id: ID!
                  model: String!
                }

                type Sensor @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Controller @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    model: String
                      @fromContext(
                        field: "$deviceCtx { ... on Sensor { model } ... on Controller { model } ... on Gateway { model } }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeviceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(model:)" but the selection is invalid: type condition "Gateway" is never used."#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }
        }
    }
}

// selected context values
mod selected_context_values {
    use super::*;

    // concrete providers
    mod concrete_providers {
        use super::*;

        test_versions! {
            // accepts equivalent selections from multiple concrete providers
            fn accepts_equivalent_selections_from_multiple_concrete_providers(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "pricing",
                        federation_schema(
                            version,
                            graphql!(r#"
                union PriceSource =
                  | RetailAccount
                  | WholesaleAccount
                  | CorporateAccount
                  | GuestQuote

                type GuestQuote {
                  id: ID!
                  note: String
                }

                type RetailAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: String!
                  region: String!
                }

                type WholesaleAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: String!
                  region: String!
                }

                type CorporateAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: String!
                  region: String!
                }

                type QuoteEngine @key(fields: "serialNumber") {
                  serialNumber: String!
                  estimateCost(
                    preferredCurrency: String
                      @fromContext(field: "$priceCtx { currency }")
                  ): Float
                }

                type Query {
                  source: PriceSource
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "exchange",
                        federation_schema(
                            version,
                            graphql!(r#"
                type ExchangeRate @key(fields: "code") {
                  code: String!
                  value: Float!
                  convert(to: String!): Float!
                }

                type Query {
                  exchangeRate(code: String!): ExchangeRate
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type QuoteEngine @join__type(graph: PRICING, key: "serialNumber") {
              serialNumber: String!
              estimateCost: Float
                @join__field(
                  graph: PRICING
                  contextArguments: [
                    {
                      context: "pricing__priceCtx"
                      name: "preferredCurrency"
                      type: "String"
                      selection: " { currency }"
                    }
                  ]
                )
            }
          "#),
                );
            }

            // accepts __typename from an object context provider
            fn accepts_typename_from_an_object_context_provider(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "catalog",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Product @key(fields: "id") @context(name: "productCtx") {
                  id: ID!
                  sku: String!
                  category: String
                  inspector: ProductInspector!
                }

                type ProductInspector @key(fields: "id") {
                  id: ID!
                  label(locale: String): String
                  identify(
                    typeName: String
                      @fromContext(field: "$productCtx { __typename }")
                  ): String!
                }

                type Query {
                  product(id: ID!): Product
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type ProductInspector @join__type(graph: CATALOG, key: "id") {
              id: ID!
              label(locale: String): String
              identify: String!
                @join__field(
                  graph: CATALOG
                  contextArguments: [
                    {
                      context: "catalog__productCtx"
                      name: "typeName"
                      type: "String"
                      selection: " { __typename }"
                    }
                  ]
                )
            }
          "#),
                );
            }

            // rejects a selection if the value type is different between the providers
            fn rejects_a_selection_if_the_value_type_is_different_between_the_providers(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "pricing",
                        federation_schema(
                            version,
                            graphql!(r#"
                union PriceSource =
                  | RetailAccount
                  | WholesaleAccount
                  | GuestQuote
                  | CorporateAccount

                type GuestQuote {
                  id: ID!
                  note: String
                }

                type RetailAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: String!
                }

                type WholesaleAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: Int!
                }

                type CorporateAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: String!
                  region: String!
                }

                type QuoteEngine @key(fields: "id") {
                  id: ID!
                  calculate(
                    currency: String
                      @fromContext(field: "$priceCtx { currency }")
                  ): Int!
                }

                type Query {
                  source: PriceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[pricing] Context "priceCtx" is used in "QuoteEngine.calculate(currency:)" but the selection is invalid: the type of the selection "Int" does not match the expected type "String""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects nullable context values for non-null @fromContext
            fn rejects_nullable_context_values_for_non_null_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "pricing",
                        federation_schema(
                            version,
                            graphql!(r#"
                type RetailAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: String
                }

                type QuoteEngine @key(fields: "id") {
                  id: ID!
                  calculate(
                    currency: String!
                      @fromContext(field: "$priceCtx { currency }")
                  ): Int!
                }

                type Query {
                  account(id: ID!): RetailAccount
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[pricing] Context "priceCtx" is used in "QuoteEngine.calculate(currency:)" but the selection is invalid: the type of the selection "String" does not match the expected type "String!""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }
        }
    }

    // interfaces and unions
    mod interfaces_and_unions {
        use super::*;

        test_versions! {
            // accepts __typename from an interface context provider
            fn accepts_typename_from_an_interface_context_provider(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Node @key(fields: "id") @context(name: "nodeCtx") {
                  id: ID!
                }

                type Sensor implements Node @key(fields: "id") {
                  id: ID!
                  label: String!
                }

                type Controller implements Node @key(fields: "id") {
                  id: ID!
                  label: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    kind: String @fromContext(field: "$nodeCtx { __typename }")
                  ): String!
                }

                type Query {
                  device: Node
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Diagnostics @join__type(graph: DEVICES, key: "id") {
              id: ID!
              inspect: String!
                @join__field(
                  graph: DEVICES
                  contextArguments: [
                    {
                      context: "devices__nodeCtx"
                      name: "kind"
                      type: "String"
                      selection: " { __typename }"
                    }
                  ]
                )
            }
          "#),
                );
            }

            // accepts a context on an interface if the interface has the selected field
            fn accepts_a_context_on_an_interface_if_the_interface_has_the_selected_field(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface MeteredDevice
                  @key(fields: "id")
                  @context(name: "meterCtx") {
                  id: ID!
                  voltage: Float!
                  amperage: Float
                }

                type Sensor implements MeteredDevice @key(fields: "id") {
                  id: ID!
                  voltage: Float!
                  amperage: Float
                }

                type Controller implements MeteredDevice @key(fields: "id") {
                  id: ID!
                  voltage: Float!
                  amperage: Float
                }

                type SafetyRelay @key(fields: "id") {
                  id: ID!
                  trip(
                    v: Float @fromContext(field: "$meterCtx { voltage }")
                  ): Int!
                }

                type Query {
                  device: MeteredDevice
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            interface MeteredDevice
              @join__type(graph: DEVICES, key: "id")
              @context(name: "devices__meterCtx") {
              id: ID!
              voltage: Float!
              amperage: Float
            }
          "#),
                );
            }

            // accepts an interface context that concrete branches resolve
            fn accepts_an_interface_context_that_concrete_branches_resolve(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface MeteredDevice
                  @key(fields: "id")
                  @context(name: "meterCtx") {
                  id: ID!
                }

                type Sensor implements MeteredDevice @key(fields: "id") {
                  id: ID!
                  reading: Float!
                }

                type Controller implements MeteredDevice @key(fields: "id") {
                  id: ID!
                  reading: Float!
                }

                type SafetyRelay @key(fields: "id") {
                  id: ID!
                  trip(
                    reading: Float
                      @fromContext(
                        field: "$meterCtx { ... on Sensor { reading } ... on Controller { reading } }"
                      )
                  ): Int!
                }

                type Query {
                  device: MeteredDevice
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
            }

            // rejects a context field on a union if one provider does not have the field
            fn rejects_a_context_field_on_a_union_if_one_provider_does_not_have_the_field(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "media",
                        federation_schema(
                            version,
                            graphql!(r#"
                union PlayableAsset @context(name: "assetCtx") = Film | Trailer

                type Film @key(fields: "id") @context(name: "assetCtx") {
                  id: ID!
                  rating: String!
                  player: Player!
                }

                type Trailer @key(fields: "id") @context(name: "assetCtx") {
                  id: ID!
                  campaign: String!
                  player: Player!
                }

                type Player @key(fields: "id") {
                  id: ID!
                  start(
                    rating: String @fromContext(field: "$assetCtx { rating }")
                  ): Int!
                }

                type Query {
                  asset: PlayableAsset
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[media] Context "assetCtx" is used in "Player.start(rating:)" but the selection is invalid for type Trailer. Error: Cannot query field "rating" on type "Trailer"."#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }
        }
    }

    // type conditions
    mod type_conditions {
        use super::*;

        test_versions! {
            // accepts explicit branch selections if the providers have different field names
            fn accepts_explicit_branch_selections_if_the_providers_have_different_field_names(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "fulfillment",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeliverySource =
                  | CourierDelivery
                  | LockerPickup
                  | DroneDelivery
                  | StorePickup

                type StorePickup {
                  id: ID!
                  desk: String
                }

                type DroneDelivery {
                  id: ID!
                  depot: String!
                }

                type CourierDelivery
                  @key(fields: "id")
                  @context(name: "deliveryCtx") {
                  id: ID!
                  courierZone: String!
                }

                type LockerPickup
                  @key(fields: "id")
                  @context(name: "deliveryCtx") {
                  id: ID!
                  lockerZone: String!
                }

                type RoutePlanner @key(fields: "id") {
                  id: ID!
                  plan(
                    zone: String
                      @fromContext(
                        field: "$deliveryCtx { ... on CourierDelivery { courierZone } ... on LockerPickup { lockerZone } }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeliverySource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type RoutePlanner @join__type(graph: FULFILLMENT, key: "id") {
              id: ID!
              plan: Int!
                @join__field(
                  graph: FULFILLMENT
                  contextArguments: [
                    {
                      context: "fulfillment__deliveryCtx"
                      name: "zone"
                      type: "String"
                      selection: " { ... on CourierDelivery { courierZone } ... on LockerPickup { lockerZone } }"
                    }
                  ]
                )
            }
          "#),
                );
            }

            // rejects type conditions that do not include a concrete context provider
            fn rejects_type_conditions_that_do_not_include_a_concrete_context_provider(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeviceSource = Sensor | Controller | Camera

                type Camera {
                  id: ID!
                  model: String!
                }

                type Sensor @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Controller @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    model: String
                      @fromContext(
                        field: "$deviceCtx { ... on Gateway { model } }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeviceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(model:)" but the selection is invalid: no type condition matches the location "Sensor""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a conditional selection if no inline fragment branch is a runtime type
            fn rejects_a_conditional_selection_if_no_inline_fragment_branch_is_a_runtime_type(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeviceSource = Sensor | Controller | Camera

                type Camera {
                  id: ID!
                  model: String!
                }

                type Sensor @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Controller @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    model: String
                      @fromContext(
                        field: "$deviceCtx { ... on Gateway { model } ... on Router { model } }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeviceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(model:)" but the selection is invalid: no type condition matches the location "Sensor""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects two branches that have the same type condition
            fn rejects_two_branches_that_have_the_same_type_condition(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                union DeviceSource = Sensor | Controller

                type Sensor @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Controller @key(fields: "id") @context(name: "deviceCtx") {
                  id: ID!
                  model: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    model: String
                      @fromContext(
                        field: "$deviceCtx { ... on Sensor { model } ... on Sensor { model } }"
                      )
                  ): Int!
                }

                type Query {
                  source: DeviceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[devices] Context "deviceCtx" is used in "Diagnostics.inspect(model:)" but the selection is invalid: type conditions have same name"#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }
        }
    }
}

// complex query-plan-shaped context selections
mod complex_query_plan_shaped_context_selections {
    use super::*;

    test_versions! {
        // checks nested list selection against '[[[String]]]'
        fn checks_nested_list_selection_against_string(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
                type Catalog @context(name: "catalogCtx") {
                  products: [[Product!]!]!
                  auditor: Auditor
                }

                type Product {
                  codes: [String]!
                }

                type Auditor @key(fields: "id") {
                  id: ID!
                  audit(codes: [[[String]]] @fromContext(field: "$catalogCtx { products { codes } }")): Boolean
                }

                type Query {
                  catalog: Catalog
                }
              "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // checks nested list selection against '[[[String]]]!'
        fn checks_nested_list_selection_against_string_2(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
                type Catalog @context(name: "catalogCtx") {
                  products: [[Product!]!]!
                  auditor: Auditor
                }

                type Product {
                  codes: [String]!
                }

                type Auditor @key(fields: "id") {
                  id: ID!
                  audit(codes: [[[String]]]! @fromContext(field: "$catalogCtx { products { codes } }")): Boolean
                }

                type Query {
                  catalog: Catalog
                }
              "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[catalog] Context "catalogCtx" is used in "Auditor.audit(codes:)" but the selection is invalid: the type of the selection "[[[String]]]" does not match the expected type "[[[String]]]!""#)
                    .code("CONTEXT_INVALID_SELECTION")],
            );
        }

        // checks nested list selection against '[[[String]!]]'
        fn checks_nested_list_selection_against_string_3(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
                type Catalog @context(name: "catalogCtx") {
                  products: [[Product!]!]!
                  auditor: Auditor
                }

                type Product {
                  codes: [String]!
                }

                type Auditor @key(fields: "id") {
                  id: ID!
                  audit(codes: [[[String]!]] @fromContext(field: "$catalogCtx { products { codes } }")): Boolean
                }

                type Query {
                  catalog: Catalog
                }
              "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[catalog] Context "catalogCtx" is used in "Auditor.audit(codes:)" but the selection is invalid: the type of the selection "[[[String]]]" does not match the expected type "[[[String]!]]""#)
                    .code("CONTEXT_INVALID_SELECTION")],
            );
        }

        // checks nested list selection against '[[String]]'
        fn checks_nested_list_selection_against_string_4(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
                type Catalog @context(name: "catalogCtx") {
                  products: [[Product!]!]!
                  auditor: Auditor
                }

                type Product {
                  codes: [String]!
                }

                type Auditor @key(fields: "id") {
                  id: ID!
                  audit(codes: [[String]] @fromContext(field: "$catalogCtx { products { codes } }")): Boolean
                }

                type Query {
                  catalog: Catalog
                }
              "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[catalog] Context "catalogCtx" is used in "Auditor.audit(codes:)" but the selection is invalid: the type of the selection "[[[String]]]" does not match the expected type "[[String]]""#)
                    .code("CONTEXT_INVALID_SELECTION")],
            );
        }

        // accepts nested field paths within conditional selections
        fn accepts_nested_field_paths_within_conditional_selections(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Catalog @context(name: "catalogCtx") {
                products: [Product!]!
                auditor: Auditor
              }

              type Product {
                sku: String!
              }

              type Auditor @key(fields: "id") {
                id: ID!
                audit(
                  codes: [String]
                    @fromContext(
                      field: "$catalogCtx { ... on Catalog { products { sku } } }"
                    )
                ): Boolean
              }

              type Query {
                catalog: Catalog
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // accepts a list context through an entity extension with multiple arguments
        fn accepts_a_list_context_through_an_entity_extension_with_multiple_arguments(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
              type ProductCatalog
                @key(fields: "id")
                @context(name: "productCtx") {
                id: ID!
                locale: String!
                products: [Product!]!
                auditor: InvoiceAuditor!
              }

              type Product @key(fields: "id") {
                id: ID!
                sku: String!
                category: ProductCategory!
              }

              type ProductCategory @key(fields: "id") {
                id: ID!
                code: String!
              }

              type InvoiceAuditor @key(fields: "id") {
                id: ID!
                audit(
                  skus: [String]
                    @fromContext(field: "$productCtx { products { sku } }")
                  locale: String @fromContext(field: "$productCtx { locale }")
                ): Float!
              }

              type Query {
                catalog(id: ID!): ProductCatalog
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "inventory",
                    federation_schema(
                        version,
                        graphql!(r#"
              extend type Product @key(fields: "id") {
                id: ID! @external
                stock: Int!
                warehouse: String!
              }

              type Query {
                inventoryStatus: String!
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "pricing",
                    federation_schema(
                        version,
                        graphql!(r#"
              type ExchangeRate @key(fields: "code") {
                code: String!
                value: Float!
                convert(to: String!): Float!
              }

              type Query {
                exchangeRate(code: String!): ExchangeRate
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
            to_contain_graphql(
                result.supergraph_sdl(),
                graphql!(r#"
          type InvoiceAuditor @join__type(graph: CATALOG, key: "id") {
            id: ID!
            audit: Float!
              @join__field(
                graph: CATALOG
                contextArguments: [
                  {
                    context: "catalog__productCtx"
                    name: "skus"
                    type: "[String]"
                    selection: " { products { sku } }"
                  }
                  {
                    context: "catalog__productCtx"
                    name: "locale"
                    type: "String"
                    selection: " { locale }"
                  }
                ]
              )
          }
        "#),
            );
        }

        // rejects an invalid field beneath a list context selection
        fn rejects_an_invalid_field_beneath_a_list_context_selection(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
              type ProductCatalog
                @key(fields: "id")
                @context(name: "productCtx") {
                id: ID!
                products: [Product!]!
              }

              type Product @key(fields: "id") {
                id: ID!
                sku: String!
              }

              type InvoiceAuditor @key(fields: "id") {
                id: ID!
                audit(
                  skus: [String]
                    @fromContext(
                      field: "$productCtx { products { reference } }"
                    )
                ): Float!
              }

              type Query {
                catalog(id: ID!): ProductCatalog
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[catalog] Context "productCtx" is used in "InvoiceAuditor.audit(skus:)" but the selection is invalid for type Product. Error: Cannot query field "reference" on type "Product"."#)
                    .code("CONTEXT_INVALID_SELECTION")],
            );
        }
    }
}

// supergraph
mod supergraph {
    use super::*;

    test_versions! {
        // does not leak federation__ContextFieldValue from service SDL
        fn does_not_leak_federation_context_field_value_from_service_sdl(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "accounts",
                    federation_schema(
                        version,
                        graphql!(r#"
              scalar federation__ContextFieldValue

              type Query {
                account: Account
              }

              type Account @context(name: "accountCtx") {
                locale: String!
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
            assert_not_to_match_string(result.supergraph_sdl(), "scalar federation__ContextFieldValue");
        }

        // materializes a nested scalar in join metadata
        fn materializes_a_nested_scalar_in_join_metadata(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "orders",
                    federation_schema(
                        version,
                        graphql!(r#"
                type RewardLedger @shareable {
                  id: ID!
                  level: String!
                  voucher: String
                  expiresAt: String
                }

                type Customer @key(fields: "id") @context(name: "customerCtx") {
                  id: ID!
                  profile: RewardLedger!
                  cart: Cart!
                }

                type Cart @key(fields: "id") {
                  id: ID!
                  checkout(
                    level: String
                      @fromContext(field: "$customerCtx { profile { level } }")
                  ): Float!
                }

                type Query {
                  customer(id: ID!): Customer
                }
              "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
            assert_not_to_match_string(result.public_sdl(), "@context");
            assert_not_to_match_string(result.public_sdl(), "@fromContext");
            assert_not_to_match_string(result.public_sdl(), "join__ContextArgument");
            to_contain_graphql(
                result.public_sdl(),
                graphql!(r#"
          type Cart {
            id: ID!
            checkout: Float!
          }
        "#),
            );
            expect_context_spec_definitions(result.supergraph_sdl());
            to_contain_graphql(
                result.supergraph_sdl(),
                graphql!(r#"
          type Cart @join__type(graph: ORDERS, key: "id") {
            id: ID!
            checkout: Float!
              @join__field(
                graph: ORDERS
                contextArguments: [
                  {
                    context: "orders__customerCtx"
                    name: "level"
                    type: "String"
                    selection: " { profile { level } }"
                  }
                ]
              )
          }
        "#),
            );
        }

        // materializes a list-valued context selection in join metadata
        fn materializes_a_list_valued_context_selection_in_join_metadata(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "warehouse",
                    federation_schema(
                        version,
                        graphql!(r#"
                type StockItem {
                  code: ID!
                  description: String
                  weightGrams: Int!
                }

                type Shelf
                  @key(fields: "shelfCode")
                  @context(name: "shelfCtx") {
                  shelfCode: String!
                  aisle: String!
                  items: [StockItem!]!
                  robot: PickRobot!
                }

                type PickRobot @key(fields: "serial") {
                  serial: ID!
                  reserveItems(
                    skus: [ID]
                      @fromContext(field: "$shelfCtx { items { code } }")
                  ): [ID!]!
                }

                type Query {
                  shelf(shelfCode: String!): Shelf
                }
              "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
            assert_not_to_match_string(result.public_sdl(), "@context");
            assert_not_to_match_string(result.public_sdl(), "@fromContext");
            assert_not_to_match_string(result.public_sdl(), "join__ContextArgument");
            to_contain_graphql(
                result.public_sdl(),
                graphql!(r#"
          type PickRobot {
            serial: ID!
            reserveItems: [ID!]!
          }
        "#),
            );
            expect_context_spec_definitions(result.supergraph_sdl());
            to_contain_graphql(
                result.supergraph_sdl(),
                graphql!(r#"
          type PickRobot @join__type(graph: WAREHOUSE, key: "serial") {
            serial: ID!
            reserveItems: [ID!]!
              @join__field(
                graph: WAREHOUSE
                contextArguments: [
                  {
                    context: "warehouse__shelfCtx"
                    name: "skus"
                    type: "[ID]"
                    selection: " { items { code } }"
                  }
                ]
              )
          }
        "#),
            );
        }

        // adds the subgraph name to each context name
        fn adds_the_subgraph_name_to_each_context_name(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "invoices",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Invoice @key(fields: "id") @context(name: "sourceCtx") {
                id: ID!
                reference: String!
              }

              type Auditor @key(fields: "id") {
                id: ID!
                auditInvoice(
                  ref: String @fromContext(field: "$sourceCtx { reference }")
                ): Int!
              }

              type Query {
                invoice(id: ID!): Invoice
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "shipments",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Parcel @key(fields: "id") @context(name: "sourceCtx") {
                id: ID!
                reference: String!
              }

              type Auditor @key(fields: "id") {
                id: ID!
                auditShipment(
                  ref: String @fromContext(field: "$sourceCtx { reference }")
                ): Int!
              }

              type Query {
                parcel(id: ID!): Parcel
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
            to_contain_graphql(
                result.supergraph_sdl(),
                graphql!(r#"
          type Invoice
            @join__type(graph: INVOICES, key: "id")
            @context(name: "invoices__sourceCtx") {
            id: ID!
            reference: String!
          }
        "#),
            );
            to_contain_graphql(
                result.supergraph_sdl(),
                graphql!(r#"
          type Parcel
            @join__type(graph: SHIPMENTS, key: "id")
            @context(name: "shipments__sourceCtx") {
            id: ID!
            reference: String!
          }
        "#),
            );
        }
    }
}

// context providers and the public API
mod context_providers_and_the_public_api {
    use super::*;

    // type and interface extensions
    mod type_and_interface_extensions {
        use super::*;

        test_versions! {
            // accepts a context on a type extension
            fn accepts_a_context_on_a_type_extension(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") {
                  id: ID!
                  plan: String!
                }

                extend type Member @context(name: "memberCtx") {
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Member
              @join__type(graph: BENEFITS, key: "id")
              @context(name: "benefits__memberCtx") {
              id: ID!
              plan: String!
              wallet: Wallet!
            }
          "#),
                );
            }

            // accepts a context on an interface extension
            fn accepts_a_context_on_an_interface_extension(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "devices",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Node @key(fields: "id") {
                  id: ID!
                }

                extend interface Node @context(name: "nodeCtx") {
                  label: String!
                }

                type Sensor implements Node @key(fields: "id") {
                  id: ID!
                  label: String!
                }

                type Diagnostics @key(fields: "id") {
                  id: ID!
                  inspect(
                    label: String @fromContext(field: "$nodeCtx { label }")
                  ): String!
                }

                type Query {
                  device: Node
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Diagnostics @join__type(graph: DEVICES, key: "id") {
              id: ID!
              inspect: String!
                @join__field(
                  graph: DEVICES
                  contextArguments: [
                    {
                      context: "devices__nodeCtx"
                      name: "label"
                      type: "String"
                      selection: " { label }"
                    }
                  ]
                )
            }
          "#),
                );
            }
        }
    }

    // resolvable keys
    mod resolvable_keys {
        use super::*;

        test_versions! {
            // accepts @fromContext on a type extension if the base type has a resolvable key
            fn accepts_from_context_on_a_type_extension_if_the_base_type_has_a_resolvable_key(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                }

                extend type Wallet {
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Wallet @join__type(graph: BENEFITS, key: "id") {
              id: ID!
              apply: Int!
                @join__field(
                  graph: BENEFITS
                  contextArguments: [
                    {
                      context: "benefits__memberCtx"
                      name: "plan"
                      type: "String"
                      selection: " { plan }"
                    }
                  ]
                )
            }
          "#),
                );
            }

            // accepts @fromContext if one of the keys is not resolvable
            fn accepts_from_context_if_one_of_the_keys_is_not_resolvable(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                }

                type Wallet
                  @key(fields: "id", resolvable: false)
                  @key(fields: "legacyId") {
                  id: ID!
                  legacyId: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Wallet
              @join__type(graph: BENEFITS, key: "id", resolvable: false)
              @join__type(graph: BENEFITS, key: "legacyId") {
              id: ID!
              legacyId: ID!
              apply: Int!
                @join__field(
                  graph: BENEFITS
                  contextArguments: [
                    {
                      context: "benefits__memberCtx"
                      name: "plan"
                      type: "String"
                      selection: " { plan }"
                    }
                  ]
                )
            }
          "#),
                );
            }

            // accepts @fromContext on a type extension if one of the base keys is not resolvable
            fn accepts_from_context_on_a_type_extension_if_one_of_the_base_keys_is_not_resolvable(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet
                  @key(fields: "id", resolvable: false)
                  @key(fields: "legacyId") {
                  id: ID!
                  legacyId: ID!
                }

                extend type Wallet {
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
            }

            // accepts @fromContext if an extension declares a resolvable key and the base type does not
            fn accepts_from_context_if_an_extension_declares_a_resolvable_key_and_the_base_type_does_not(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id", resolvable: false) {
                  id: ID!
                  legacyId: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                extend type Wallet @key(fields: "legacyId")

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
            }

            // rejects @fromContext if the only key of the object is not resolvable
            fn rejects_from_context_if_the_only_key_of_the_object_is_not_resolvable(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id", resolvable: false) {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Object "Wallet" has no resolvable key but has a field with a contextual argument."#)
                        .code("CONTEXT_NO_RESOLVABLE_KEY")],
                );
            }

            // rejects @fromContext on a type extension if the base type key is not resolvable
            fn rejects_from_context_on_a_type_extension_if_the_base_type_key_is_not_resolvable(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id", resolvable: false) {
                  id: ID!
                }

                extend type Wallet {
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Object "Wallet" has no resolvable key but has a field with a contextual argument."#)
                        .code("CONTEXT_NO_RESOLVABLE_KEY")],
                );
            }

            // rejects @fromContext on a base type if only the type extension declares a resolvable key
            fn rejects_from_context_on_a_base_type_if_only_the_type_extension_declares_a_resolvable_key(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                }

                type Wallet {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                extend type Wallet @key(fields: "id", resolvable: false)

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Object "Wallet" has no resolvable key but has a field with a contextual argument."#)
                        .code("CONTEXT_NO_RESOLVABLE_KEY")],
                );
            }
        }
    }

    // the public API schema
    mod the_public_api_schema {
        use super::*;

        test_versions! {
            // removes @fromContext if another graph defines the same nullable argument
            fn removes_from_context_if_another_graph_defines_the_same_nullable_argument(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") @shareable {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "payments",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Wallet @key(fields: "id") @shareable {
                  id: ID!
                  apply(plan: String): Int!
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.public_sdl(),
                    graphql!(r#"
            type Wallet {
              id: ID!
              apply: Int!
            }
          "#),
                );
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Wallet
              @join__type(graph: BENEFITS, key: "id")
              @join__type(graph: PAYMENTS, key: "id") {
              id: ID!
              apply: Int!
                @join__field(
                  graph: BENEFITS
                  contextArguments: [
                    {
                      context: "benefits__memberCtx"
                      name: "plan"
                      type: "String"
                      selection: " { plan }"
                    }
                  ]
                )
                @join__field(graph: PAYMENTS)
            }
          "#),
                );
            }

            // removes @fromContext if another graph gives the argument a default value
            fn removes_from_context_if_another_graph_gives_the_argument_a_default_value(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") @shareable {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "payments",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Wallet @key(fields: "id") @shareable {
                  id: ID!
                  apply(plan: String = "standard"): Int!
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.public_sdl(),
                    graphql!(r#"
            type Wallet {
              id: ID!
              apply: Int!
            }
          "#),
                );
            }

            // rejects a required argument if another graph makes the same argument contextual
            fn rejects_a_required_argument_if_another_graph_makes_the_same_argument_contextual(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") @shareable {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "payments",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Wallet @key(fields: "id") @shareable {
                  id: ID!
                  apply(plan: String!): Int!
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"Argument "Wallet.apply(plan:)" is contextual in at least one subgraph but in "Wallet.apply(plan:)" it does not have @fromContext, is not nullable and has no default value."#)
                        .code("CONTEXTUAL_ARGUMENT_NOT_CONTEXTUAL_IN_ALL_SUBGRAPHS")],
                );
            }
        }
    }

    // @interfaceObject providers
    mod interface_object_providers {
        use super::*;

        test_versions! {
            // rejects a provider type that has @interfaceObject
            fn rejects_a_provider_type_that_has_interface_object(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Benefit
                  @interfaceObject
                  @key(fields: "id")
                  @context(name: "benefitCtx") {
                  id: ID!
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$benefitCtx { plan }")
                  ): Int!
                }

                type Query {
                  benefit(id: ID!): Benefit
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "catalog",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Benefit @key(fields: "id") {
                  id: ID!
                }

                type Coupon implements Benefit @key(fields: "id") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  noop: Int
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Context "benefitCtx" is used in "Wallet.apply(plan:)" but the selection is invalid: One of the types in the selection is an interfaceObject: "Benefit""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a provider type extension that has @interfaceObject
            fn rejects_a_provider_type_extension_that_has_interface_object(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Benefit @key(fields: "id") {
                  id: ID!
                }

                extend type Benefit
                  @interfaceObject
                  @context(name: "benefitCtx") {
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$benefitCtx { plan }")
                  ): Int!
                }

                type Query {
                  benefit(id: ID!): Benefit
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "catalog",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Benefit @key(fields: "id") {
                  id: ID!
                }

                type Coupon implements Benefit @key(fields: "id") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  noop: Int
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Context "benefitCtx" is used in "Wallet.apply(plan:)" but the selection is invalid: One of the types in the selection is an interfaceObject: "Benefit""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a provider if @interfaceObject is on the definition and @context is on an extension
            fn rejects_a_provider_if_interface_object_is_on_the_definition_and_context_is_on_an_extension(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Benefit @interfaceObject @key(fields: "id") {
                  id: ID!
                }

                extend type Benefit @context(name: "benefitCtx") {
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$benefitCtx { plan }")
                  ): Int!
                }

                type Query {
                  benefit(id: ID!): Benefit
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "catalog",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Benefit @key(fields: "id") {
                  id: ID!
                }

                type Coupon implements Benefit @key(fields: "id") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  noop: Int
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Context "benefitCtx" is used in "Wallet.apply(plan:)" but the selection is invalid: One of the types in the selection is an interfaceObject: "Benefit""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a provider if @context is on the definition and @interfaceObject is on an extension
            fn rejects_a_provider_if_context_is_on_the_definition_and_interface_object_is_on_an_extension(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Benefit @key(fields: "id") @context(name: "benefitCtx") {
                  id: ID!
                  plan: String!
                  wallet: Wallet!
                }

                extend type Benefit @interfaceObject

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$benefitCtx { plan }")
                  ): Int!
                }

                type Query {
                  benefit(id: ID!): Benefit
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "catalog",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Benefit @key(fields: "id") {
                  id: ID!
                }

                type Coupon implements Benefit @key(fields: "id") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  noop: Int
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Context "benefitCtx" is used in "Wallet.apply(plan:)" but the selection is invalid: One of the types in the selection is an interfaceObject: "Benefit""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }

            // rejects a provider that has @interfaceObject if an extension has @fromContext
            fn rejects_a_provider_that_has_interface_object_if_an_extension_has_from_context(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Benefit
                  @interfaceObject
                  @key(fields: "id")
                  @context(name: "benefitCtx") {
                  id: ID!
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                }

                extend type Wallet {
                  apply(
                    plan: String @fromContext(field: "$benefitCtx { plan }")
                  ): Int!
                }

                type Query {
                  benefit(id: ID!): Benefit
                }
              "#),
                            &[],
                        ),
                    ),
                    service(
                        "catalog",
                        federation_schema(
                            version,
                            graphql!(r#"
                interface Benefit @key(fields: "id") {
                  id: ID!
                }

                type Coupon implements Benefit @key(fields: "id") {
                  id: ID!
                  plan: String!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                }

                type Query {
                  noop: Int
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[benefits] Context "benefitCtx" is used in "Wallet.apply(plan:)" but the selection is invalid: One of the types in the selection is an interfaceObject: "Benefit""#)
                        .code("CONTEXT_INVALID_SELECTION")],
                );
            }
        }
    }
}

// composition across subgraph boundaries
mod composition_across_subgraph_boundaries {
    use super::*;

    test_versions! {
        // accepts an external field if the same subgraph declares the context
        fn accepts_an_external_field_if_the_same_subgraph_declares_the_context(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                wallet: Wallet!
                plan: String! @external
              }

              type Wallet @key(fields: "id") {
                id: ID!
                apply(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int!
              }

              type Query {
                member(id: ID!): Member
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "identity",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") {
                id: ID!
                plan: String!
              }

              type Wallet @key(fields: "id") {
                id: ID!
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // marks an external field as used if only a context selection refers to it
        fn marks_an_external_field_as_used_if_only_a_context_selection_refers_to_it(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String! @external
                wallet: Wallet!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                apply(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int!
              }

              type Query {
                member(id: ID!): Member
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "identity",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") {
                id: ID!
                plan: String!
              }

              type Wallet @key(fields: "id") {
                id: ID!
              }

              type Query {
                noop: Int
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
            to_contain_graphql(
                result.supergraph_sdl(),
                graphql!(r#"
          type Member
            @join__type(graph: BENEFITS, key: "id")
            @join__type(graph: IDENTITY, key: "id")
            @context(name: "benefits__memberCtx") {
            id: ID!
            plan: String!
              @join__field(graph: BENEFITS, external: true)
              @join__field(graph: IDENTITY)
            wallet: Wallet! @join__field(graph: BENEFITS)
          }
        "#),
            );
        }

        // rejects a context declaration from another sybgraph
        fn rejects_a_context_declaration_from_another_sybgraph(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "identity",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
              }

              type Query {
                member(id: ID!): Member
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") {
                id: ID! @external
                plan: String! @external
                wallet: Wallet!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                apply(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int!
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[benefits] Context "memberCtx" is used at location "Wallet.apply(plan:)" but is never set."#)
                    .code("CONTEXT_NOT_SET")],
            );
        }

        // rejects a subgraph that uses @fromContext but does not declare the context
        fn rejects_a_subgraph_that_uses_from_context_but_does_not_declare_the_context(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "identity",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                age: Int!
              }

              type Query {
                member(id: ID!): Member
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "analytics",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") {
                id: ID! @external
                wallet: Wallet!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                measure(
                  age: Int @fromContext(field: "$memberCtx { age }")
                ): Int!
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[ErrorMatcher::new()
                    .message(r#"[analytics] Context "memberCtx" is used at location "Wallet.measure(age:)" but is never set."#)
                    .code("CONTEXT_NOT_SET")],
            );
        }
    }
}

// transitive security requirements
mod transitive_security_requirements {
    use super::*;

    // @authenticated
    mod authenticated {
        use super::*;

        test_versions! {
            // accepts authenticated context data if the field is also authenticated
            fn accepts_authenticated_context_data_if_the_field_is_also_authenticated(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member
                  @key(fields: "id")
                  @context(name: "memberCtx")
                  @authenticated {
                  id: ID!
                  plan: String! @authenticated
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int! @authenticated
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Wallet @join__type(graph: IDENTITY, key: "id") {
              id: ID!
              apply: Int!
                @join__field(
                  graph: IDENTITY
                  contextArguments: [
                    {
                      context: "identity__memberCtx"
                      name: "plan"
                      type: "String"
                      selection: " { plan }"
                    }
                  ]
                )
                @authenticated
            }
          "#),
                );
            }

            // accepts authenticated conditional branches if the field is also authenticated
            fn accepts_authenticated_conditional_branches_if_the_field_is_also_authenticated(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "pricing",
                        federation_schema(
                            version,
                            graphql!(r#"
                union PriceSource =
                  | RetailAccount
                  | WholesaleAccount
                  | GuestQuote

                type GuestQuote {
                  id: ID!
                  note: String
                }

                type RetailAccount
                  @key(fields: "id")
                  @context(name: "priceCtx")
                  @authenticated {
                  id: ID!
                  retailCurrency: String! @authenticated
                }

                type WholesaleAccount
                  @key(fields: "id")
                  @context(name: "priceCtx")
                  @authenticated {
                  id: ID!
                  wholesaleCurrency: String! @authenticated
                }

                type QuoteEngine @key(fields: "id") {
                  id: ID!
                  calculate(
                    currency: String
                      @fromContext(
                        field: "$priceCtx { ... on RetailAccount { retailCurrency } ... on WholesaleAccount { wholesaleCurrency } }"
                      )
                  ): Int! @authenticated
                }

                type Query {
                  source: PriceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type QuoteEngine @join__type(graph: PRICING, key: "id") {
              id: ID!
              calculate: Int!
                @join__field(
                  graph: PRICING
                  contextArguments: [
                    {
                      context: "pricing__priceCtx"
                      name: "currency"
                      type: "String"
                      selection: " { ... on RetailAccount { retailCurrency } ... on WholesaleAccount { wholesaleCurrency } }"
                    }
                  ]
                )
                @authenticated
            }
          "#),
                );
            }

            // rejects a field that reads authenticated context data but has no auth requirement
            fn rejects_a_field_that_reads_authenticated_context_data_but_has_no_auth_requirement(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member
                  @key(fields: "id")
                  @context(name: "memberCtx")
                  @authenticated {
                  id: ID!
                  plan: String! @authenticated
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[identity] Field "Wallet.apply" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context identity__memberCtx from @fromContext selection set."#)
                        .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")],
                );
            }

            // rejects a field if the context provider type is authenticated
            fn rejects_a_field_if_the_context_provider_type_is_authenticated(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member
                  @key(fields: "id")
                  @context(name: "memberCtx")
                  @authenticated {
                  id: ID!
                  plan: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[identity] Field "Wallet.apply" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context identity__memberCtx from @fromContext selection set."#)
                        .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")],
                );
            }

            // examines every @fromContext of a field, not only the last one
            fn examines_every_from_context_of_a_field_not_only_the_last_one(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member @key(fields: "id") @context(name: "memberCtx") {
                  id: ID!
                  plan: String! @authenticated
                  tier: String!
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                    tier: String @fromContext(field: "$memberCtx { tier }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[identity] Field "Wallet.apply" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context identity__memberCtx from @fromContext selection set."#)
                        .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")],
                );
            }
        }
    }

    // @requiresScopes
    mod requires_scopes {
        use super::*;

        test_versions! {
            // accepts context data if the field has the merged @requiresScopes requirements
            fn accepts_context_data_if_the_field_has_the_merged_requires_scopes_requirements(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member
                  @key(fields: "id")
                  @context(name: "memberCtx")
                  @requiresScopes(scopes: [["member:read"]]) {
                  id: ID!
                  plan: String! @requiresScopes(scopes: [["plan:read"]])
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                    @requiresScopes(scopes: [["member:read", "plan:read"]])
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &["@requiresScopes"],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Wallet @join__type(graph: IDENTITY, key: "id") {
              id: ID!
              apply: Int!
                @join__field(
                  graph: IDENTITY
                  contextArguments: [
                    {
                      context: "identity__memberCtx"
                      name: "plan"
                      type: "String"
                      selection: " { plan }"
                    }
                  ]
                )
                @requiresScopes(scopes: [["member:read", "plan:read"]])
            }
          "#),
                );
            }

            // rejects a field that has no scope for context data with @requiresScopes
            fn rejects_a_field_that_has_no_scope_for_context_data_with_requires_scopes(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member
                  @key(fields: "id")
                  @context(name: "memberCtx")
                  @requiresScopes(scopes: [["member:read"]]) {
                  id: ID!
                  plan: String! @requiresScopes(scopes: [["plan:read"]])
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &["@requiresScopes"],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[identity] Field "Wallet.apply" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context identity__memberCtx from @fromContext selection set."#)
                        .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")],
                );
            }
        }
    }

    // @policy
    mod policy {
        use super::*;

        test_versions! {
            // accepts context data if the field has the merged @policy requirements
            fn accepts_context_data_if_the_field_has_the_merged_policy_requirements(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member
                  @key(fields: "id")
                  @context(name: "memberCtx")
                  @policy(policies: [["member_policy"]]) {
                  id: ID!
                  plan: String! @policy(policies: [["plan_policy"]])
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int! @policy(policies: [["member_policy", "plan_policy"]])
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &["@policy"],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type Wallet @join__type(graph: IDENTITY, key: "id") {
              id: ID!
              apply: Int!
                @join__field(
                  graph: IDENTITY
                  contextArguments: [
                    {
                      context: "identity__memberCtx"
                      name: "plan"
                      type: "String"
                      selection: " { plan }"
                    }
                  ]
                )
                @policy(policies: [["member_policy", "plan_policy"]])
            }
          "#),
                );
            }

            // rejects a field that has no policy for context data with @policy
            fn rejects_a_field_that_has_no_policy_for_context_data_with_policy(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "identity",
                        federation_schema(
                            version,
                            graphql!(r#"
                type Member
                  @key(fields: "id")
                  @context(name: "memberCtx")
                  @policy(policies: [["member_policy"]]) {
                  id: ID!
                  plan: String! @policy(policies: [["plan_policy"]])
                  wallet: Wallet!
                }

                type Wallet @key(fields: "id") {
                  id: ID!
                  apply(
                    plan: String @fromContext(field: "$memberCtx { plan }")
                  ): Int!
                }

                type Query {
                  member(id: ID!): Member
                }
              "#),
                            &["@policy"],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[identity] Field "Wallet.apply" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context identity__memberCtx from @fromContext selection set."#)
                        .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")],
                );
            }
        }
    }

    // more than one provider
    mod more_than_one_provider {
        use super::*;

        test_versions! {
            // accepts context data from many providers if the field auth covers every path
            fn accepts_context_data_from_many_providers_if_the_field_auth_covers_every_path(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "pricing",
                        federation_schema(
                            version,
                            graphql!(r#"
                union PriceSource =
                  | RetailAccount
                  | WholesaleAccount
                  | GuestQuote

                type GuestQuote {
                  id: ID!
                  note: String
                }

                type RetailAccount
                  @key(fields: "id")
                  @context(name: "priceCtx")
                  @authenticated {
                  id: ID!
                  currency: String! @authenticated
                }

                type WholesaleAccount
                  @key(fields: "id")
                  @context(name: "priceCtx")
                  @authenticated {
                  id: ID!
                  currency: String! @authenticated
                }

                type QuoteEngine @key(fields: "id") {
                  id: ID!
                  calculate(
                    currency: String
                      @fromContext(field: "$priceCtx { currency }")
                  ): Int! @authenticated
                }

                type Query {
                  source: PriceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_success(&result);
                to_contain_graphql(
                    result.supergraph_sdl(),
                    graphql!(r#"
            type QuoteEngine @join__type(graph: PRICING, key: "id") {
              id: ID!
              calculate: Int!
                @join__field(
                  graph: PRICING
                  contextArguments: [
                    {
                      context: "pricing__priceCtx"
                      name: "currency"
                      type: "String"
                      selection: " { currency }"
                    }
                  ]
                )
                @authenticated
            }
          "#),
                );
            }

            // rejects a field if only one provider path has an auth requirement
            fn rejects_a_field_if_only_one_provider_path_has_an_auth_requirement(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "pricing",
                        federation_schema(
                            version,
                            graphql!(r#"
                union PriceSource =
                  | RetailAccount
                  | WholesaleAccount
                  | GuestQuote

                type GuestQuote {
                  id: ID!
                  note: String
                }

                type RetailAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  currency: String!
                }

                type WholesaleAccount
                  @key(fields: "id")
                  @context(name: "priceCtx")
                  @authenticated {
                  id: ID!
                  currency: String! @authenticated
                }

                type QuoteEngine @key(fields: "id") {
                  id: ID!
                  calculate(
                    currency: String
                      @fromContext(field: "$priceCtx { currency }")
                  ): Int!
                }

                type Query {
                  source: PriceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[pricing] Field "QuoteEngine.calculate" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context pricing__priceCtx from @fromContext selection set."#)
                        .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")],
                );
            }

            // rejects a field if a conditional provider branch has an auth requirement
            fn rejects_a_field_if_a_conditional_provider_branch_has_an_auth_requirement(api, version) if ">= v2.8" {
                let result = api.compose_services(&[
                    service(
                        "pricing",
                        federation_schema(
                            version,
                            graphql!(r#"
                union PriceSource =
                  | RetailAccount
                  | WholesaleAccount
                  | GuestQuote

                type GuestQuote {
                  id: ID!
                  note: String
                }

                type RetailAccount
                  @key(fields: "id")
                  @context(name: "priceCtx") {
                  id: ID!
                  retailCurrency: String!
                }

                type WholesaleAccount
                  @key(fields: "id")
                  @context(name: "priceCtx")
                  @authenticated {
                  id: ID!
                  wholesaleCurrency: String! @authenticated
                }

                type QuoteEngine @key(fields: "id") {
                  id: ID!
                  calculate(
                    currency: String
                      @fromContext(
                        field: "$priceCtx { ... on RetailAccount { retailCurrency } ... on WholesaleAccount { wholesaleCurrency } }"
                      )
                  ): Int!
                }

                type Query {
                  source: PriceSource
                }
              "#),
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[ErrorMatcher::new()
                        .message(r#"[pricing] Field "QuoteEngine.calculate" does not specify necessary @authenticated, @requiresScopes and/or @policy auth requirements to access the transitive data in context pricing__priceCtx from @fromContext selection set."#)
                        .code("MISSING_TRANSITIVE_AUTH_REQUIREMENTS")],
                );
            }
        }
    }
}

// context reachability
mod context_reachability {
    use super::*;

    test_versions! {
        // rejects a context set by the parent type of the consuming field
        fn rejects_a_context_set_by_the_parent_type_of_the_consuming_field(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              member {
                discount
              }
            }
           "#,
                    &[&context_not_found("benefits", "Member", "discount")],
                )],
            );
        }

        // rejects a context set by the parent type even if the type can be its own ancestor
        fn rejects_a_context_set_by_the_parent_type_even_if_the_type_can_be_its_own_ancestor(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                referrer: Member
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              member {
                discount
              }
            }
           "#,
                    &[&context_not_found("benefits", "Member", "discount")],
                )],
            );
        }

        // accepts a context set by an ancestor
        fn accepts_a_context_set_by_an_ancestor(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // accepts a context set by a grandparent
        fn accepts_a_context_set_by_a_grandparent(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                card: Card
              }

              type Card @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // rejects a context that is not set on every path to the consuming field
        fn rejects_a_context_that_is_not_set_on_every_path_to_the_consuming_field(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
                wallet: Wallet
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              wallet {
                discount
              }
            }
           "#,
                    &[&context_not_found("benefits", "Wallet", "discount")],
                )],
            );
        }

        // rejects a context whose provider is never on the query path
        fn rejects_a_context_whose_provider_is_never_on_the_query_path(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                wallet: Wallet
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              wallet {
                discount
              }
            }
           "#,
                    &[&context_not_found("benefits", "Wallet", "discount")],
                )],
            );
        }

        // accepts a consumer that is not reachable from the root types
        fn accepts_a_consumer_that_is_not_reachable_from_the_root_types(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // accepts an ancestor reached through another subgraph
        fn accepts_an_ancestor_reached_through_another_subgraph(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") {
                id: ID!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // rejects a context of the same name set by another subgraph
        fn rejects_a_context_of_the_same_name_set_by_another_subgraph(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "catalog",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Tier @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                tier: Tier
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              member {
                wallet {
                  discount
                }
              }
            }
           "#,
                    &[r#"- from subgraph "catalog": cannot find field "Wallet.discount"."#, &context_not_found("benefits", "Wallet", "discount")],
                )],
            );
        }

        // accepts a context set by an interface the ancestor implements
        fn accepts_a_context_set_by_an_interface_the_ancestor_implements(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              interface Account @context(name: "memberCtx") {
                plan: String!
                wallet: Wallet
              }

              type Member implements Account @key(fields: "id") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                account: Account
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // accepts a context set by the implementation of an ancestor interface
        fn accepts_a_context_set_by_the_implementation_of_an_ancestor_interface(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              interface Account {
                wallet: Wallet
              }

              type Member implements Account
                @key(fields: "id")
                @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                account: Account
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // accepts a context set by a union the ancestor is a member of
        fn accepts_a_context_set_by_a_union_the_ancestor_is_a_member_of(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              union Account @context(name: "memberCtx") = Member

              type Member @key(fields: "id") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String
                    @fromContext(field: "$memberCtx { ... on Member { plan } }")
                ): Int
              }

              type Query {
                account: Account
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // accepts a context set by a root type
        fn accepts_a_context_set_by_a_root_type(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Query @context(name: "memberCtx") {
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // rejects a field when only one of its contexts is set by an ancestor
        fn rejects_a_field_when_only_one_of_its_contexts_is_set_by_an_ancestor(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Tier @key(fields: "id") @context(name: "tierCtx") {
                id: ID!
                level: String!
              }

              type Wallet @key(fields: "id") {
                id: ID!
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                  level: String @fromContext(field: "$tierCtx { level }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              member {
                wallet {
                  discount
                }
              }
            }
           "#,
                    &[&context_not_found("benefits", "Wallet", "discount")],
                )],
            );
        }

        // prints the query path without the contextual arguments
        fn prints_the_query_path_without_the_contextual_arguments(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                discount(
                  currency: String
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              member {
                discount(currency: "A string value")
              }
            }
           "#,
                    &[&context_not_found("benefits", "Member", "discount")],
                )],
            );
        }

        // rejects an unreachable context no matter the order of the types
        fn rejects_an_unreachable_context_no_matter_the_order_of_the_types(api, version) if ">= v2.8" {
            let reachable_first = graphql!(r#"
        type Member @key(fields: "id") @context(name: "memberCtx") {
          id: ID!
          plan: String!
          wallet: Wallet
        }

        type Wallet @key(fields: "id") {
          id: ID!
          discount(plan: String @fromContext(field: "$memberCtx { plan }")): Int
        }

        type Query {
          member: Member
          wallet: Wallet
        }
      "#);
            let unreachable_first = graphql!(r#"
        type Query {
          wallet: Wallet
          member: Member
        }

        type Wallet @key(fields: "id") {
          id: ID!
          discount(plan: String @fromContext(field: "$memberCtx { plan }")): Int
        }

        type Member @key(fields: "id") @context(name: "memberCtx") {
          id: ID!
          plan: String!
          wallet: Wallet
        }
      "#);
            for type_defs in [reachable_first, unreachable_first] {
                let result = api.compose_services(&[
                    service(
                        "benefits",
                        federation_schema(
                            version,
                            type_defs,
                            &[],
                        ),
                    ),
                ]);

                assert_composition_failure(&result);
                assert_errors_contain(
                    result.errors(),
                    &[unsatisfiable(
                        r#"
              {
                wallet {
                  discount
                }
              }
             "#,
                        &[&context_not_found("benefits", "Wallet", "discount")],
                    )],
                );
            }
        }

        // accepts a consumer reached through a @provides
        fn accepts_a_consumer_reached_through_a_provides(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "benefits",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet @provides(fields: "label")
              }

              type Wallet @key(fields: "id") {
                id: ID!
                label: String @external
                discount(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &["@provides"],
                    ),
                ),
                service(
                    "wallets",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Wallet @key(fields: "id") {
                id: ID!
                label: String @shareable
              }
            "#),
                        &[],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // rejects a @requires that can only be satisfied by a contextual field
        fn rejects_a_requires_that_can_only_be_satisfied_by_a_contextual_field(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "pricing",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                price(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &["@requires"],
                    ),
                ),
                service(
                    "shipping",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Wallet @key(fields: "id") {
                id: ID!
                price(plan: String): Int @external
                discount: Int @requires(fields: "price")
              }

              type Query {
                noop: Int
              }
            "#),
                        &["@requires"],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              member {
                wallet {
                  discount
                }
              }
            }
           "#,
                    &[r#"- from subgraph "pricing": cannot find field "Wallet.discount"."#, r#"- from subgraph "shipping": cannot satisfy @require conditions on field "Wallet.discount"."#],
                )],
            );
        }

        // accepts a @requires satisfied by a plain field
        fn accepts_a_requires_satisfied_by_a_plain_field(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "pricing",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                price(plan: String): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &["@requires"],
                    ),
                ),
                service(
                    "shipping",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Wallet @key(fields: "id") {
                id: ID!
                price(plan: String): Int @external
                discount: Int @requires(fields: "price")
              }

              type Query {
                noop: Int
              }
            "#),
                        &["@requires"],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // rejects a @requires that needs a context set on the query root
        fn rejects_a_requires_that_needs_a_context_set_on_the_query_root(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "pricing",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Query @context(name: "queryCtx") {
                locale: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                price(
                  locale: String @fromContext(field: "$queryCtx { locale }")
                ): Int
              }
            "#),
                        &["@requires"],
                    ),
                ),
                service(
                    "shipping",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Wallet @key(fields: "id") {
                id: ID!
                price(locale: String): Int @external
                discount: Int @requires(fields: "price")
              }

              type Query {
                noop: Int
              }
            "#),
                        &["@requires"],
                    ),
                ),
            ]);

            assert_composition_failure(&result);
            assert_errors_contain(
                result.errors(),
                &[unsatisfiable(
                    r#"
            {
              wallet {
                discount
              }
            }
           "#,
                    &[r#"- from subgraph "pricing": cannot find field "Wallet.discount"."#, r#"- from subgraph "shipping": cannot satisfy @require conditions on field "Wallet.discount"."#],
                )],
            );
        }

        // accepts a @requires selection that sets its own context
        fn accepts_a_requires_selection_that_sets_its_own_context(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "pricing",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                price(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int
              }

              type Query {
                member: Member
              }
            "#),
                        &["@requires"],
                    ),
                ),
                service(
                    "shipping",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") {
                id: ID!
                wallet: Wallet @external
                discount: Int @requires(fields: "wallet { price }")
              }

              type Wallet @key(fields: "id") {
                id: ID!
                price(plan: String): Int @external
              }

              type Query {
                noop: Int
              }
            "#),
                        &["@requires"],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }

        // accepts a field with a contextual argument and a plain @requires
        fn accepts_a_field_with_a_contextual_argument_and_a_plain_requires(api, version) if ">= v2.8" {
            let result = api.compose_services(&[
                service(
                    "pricing",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Member @key(fields: "id") @context(name: "memberCtx") {
                id: ID!
                plan: String!
                wallet: Wallet
              }

              type Wallet @key(fields: "id") {
                id: ID!
                points: Int @external
                reward(
                  plan: String @fromContext(field: "$memberCtx { plan }")
                ): Int @requires(fields: "points")
              }

              type Query {
                member: Member
              }
            "#),
                        &["@requires"],
                    ),
                ),
                service(
                    "shipping",
                    federation_schema(
                        version,
                        graphql!(r#"
              type Wallet @key(fields: "id") {
                id: ID!
                points: Int
              }

              type Query {
                noop: Int
              }
            "#),
                        &["@requires"],
                    ),
                ),
            ]);

            assert_composition_success(&result);
        }
    }
}
