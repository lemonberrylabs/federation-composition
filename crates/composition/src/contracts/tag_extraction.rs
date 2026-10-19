//! Port of `src/contracts/tag-extraction.ts`.
//!
//! NOTE(port): every `visit` in the TS has callbacks only on type definition and extension
//! kinds (which only occur in `document.definitions`), and the callbacks either return a
//! replacement node or `false`; the port maps or iterates `document.definitions` in order,
//! which visits the same nodes in the same order.

use std::rc::Rc;

use graphql_js::{
    ConstDirective, Definition, Directive as DirectiveNode, Document, EnumValueDefinition, FieldDefinition,
    InputValueDefinition, Name, OperationType, Value,
};
use crate::utils::collections::{IndexMap, IndexSet};

use crate::types::ServiceDefinition;
use crate::utils::js::{JsError, JsResult};
use crate::utils::link::extract_link_implementations;

/// `TagExtractionStrategy = (directiveNode) => string | null`.
pub type TagExtractionStrategy = Rc<dyn Fn(&DirectiveNode) -> Option<String>>;

/// The function returned by `createTransformTagDirectives(tagDirectiveName,
/// inaccessibleDirectiveName)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransformTagDirectives {
    tag_directive_name: String,
    inaccessible_directive_name: String,
}

fn create_transform_tag_directives(tag_directive_name: &str, inaccessible_directive_name: &str) -> TransformTagDirectives {
    TransformTagDirectives {
        tag_directive_name: tag_directive_name.to_string(),
        inaccessible_directive_name: inaccessible_directive_name.to_string(),
    }
}

impl TransformTagDirectives {
    /// `createTransformTagDirectives(tagDirectiveName, inaccessibleDirectiveName)`.
    pub fn new(tag_directive_name: &str, inaccessible_directive_name: &str) -> Self {
        create_transform_tag_directives(tag_directive_name, inaccessible_directive_name)
    }

    pub fn tag_directive_name(&self) -> &str {
        &self.tag_directive_name
    }

    pub fn inaccessible_directive_name(&self) -> &str {
        &self.inaccessible_directive_name
    }

    /// `transformTagDirectives(node, includeInaccessibleDirective = false)`: the node's
    /// directives without `@tag`, plus `@inaccessible` if requested and missing.
    pub fn call(&self, directives: &[ConstDirective], include_inaccessible_directive: bool) -> Vec<ConstDirective> {
        let mut has_inaccessible_directive = false;
        let mut directives: Vec<ConstDirective> = directives
            .iter()
            .filter(|directive| {
                if directive.name.value == self.inaccessible_directive_name {
                    has_inaccessible_directive = true;
                }
                directive.name.value != self.tag_directive_name
            })
            .cloned()
            .collect();

        if !has_inaccessible_directive && include_inaccessible_directive {
            directives.push(DirectiveNode::new(self.inaccessible_directive_name.clone(), Vec::new()));
        }

        directives
    }
}

/// `makeTypesFromSetInaccessible(documentNode, types, transformTagDirectives)`.
fn make_types_from_set_inaccessible(
    document_node: &Document,
    types: &IndexSet<String>,
    transform_tag_directives: &TransformTagDirectives,
) -> Document {
    // We can only apply `@inaccessible` once on each unique typename, otherwise we get a composition error
    let mut already_applied_on_type_names: IndexSet<String> = IndexSet::default();

    // `typeHandler` on object, interface, input object and enum definitions and extensions.
    let definitions = document_node
        .definitions
        .iter()
        .map(|definition| {
            let mut definition = definition.clone();
            let parts = match &mut definition {
                Definition::ObjectTypeExtension(n) => Some((&n.name.value, &mut n.directives)),
                Definition::ObjectTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::InterfaceTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::InterfaceTypeExtension(n) => Some((&n.name.value, &mut n.directives)),
                Definition::InputObjectTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::InputObjectTypeExtension(n) => Some((&n.name.value, &mut n.directives)),
                Definition::EnumTypeDefinition(n) => Some((&n.name.value, &mut n.directives)),
                Definition::EnumTypeExtension(n) => Some((&n.name.value, &mut n.directives)),
                _ => None,
            };
            if let Some((name, directives)) = parts {
                if types.contains(name.as_str()) && !already_applied_on_type_names.contains(name.as_str()) {
                    already_applied_on_type_names.insert(name.clone());
                    *directives = transform_tag_directives.call(directives, true);
                }
            }
            definition
        })
        .collect();
    Document { definitions }
}

/// The parts of a type definition or extension that `TypeDefinitionHandler` reads.
struct TypeDefinitionParts<'a> {
    name: &'a str,
    directives: &'a [ConstDirective],
    /// `fields` of object and interface types (`FieldDefinition`s, which have `arguments`).
    field_definitions: &'a [FieldDefinition],
    /// `fields` of input object types (`InputValueDefinition`s, no `arguments`).
    input_fields: &'a [InputValueDefinition],
    values: &'a [EnumValueDefinition],
}

/// The kinds `collectTagsBySchemaCoordinateFromSubgraph` visits, with their parts.
fn type_definition_parts(definition: &Definition) -> Option<TypeDefinitionParts<'_>> {
    fn parts<'a>(name: &'a Name, directives: &'a [ConstDirective]) -> TypeDefinitionParts<'a> {
        TypeDefinitionParts {
            name: &name.value,
            directives,
            field_definitions: &[],
            input_fields: &[],
            values: &[],
        }
    }
    Some(match definition {
        Definition::ScalarTypeDefinition(n) => parts(&n.name, &n.directives),
        Definition::ScalarTypeExtension(n) => parts(&n.name, &n.directives),
        Definition::UnionTypeDefinition(n) => parts(&n.name, &n.directives),
        Definition::UnionTypeExtension(n) => parts(&n.name, &n.directives),
        Definition::ObjectTypeDefinition(n) => TypeDefinitionParts {
            field_definitions: &n.fields,
            ..parts(&n.name, &n.directives)
        },
        Definition::ObjectTypeExtension(n) => TypeDefinitionParts {
            field_definitions: &n.fields,
            ..parts(&n.name, &n.directives)
        },
        Definition::InterfaceTypeDefinition(n) => TypeDefinitionParts {
            field_definitions: &n.fields,
            ..parts(&n.name, &n.directives)
        },
        Definition::InterfaceTypeExtension(n) => TypeDefinitionParts {
            field_definitions: &n.fields,
            ..parts(&n.name, &n.directives)
        },
        Definition::InputObjectTypeDefinition(n) => TypeDefinitionParts {
            input_fields: &n.fields,
            ..parts(&n.name, &n.directives)
        },
        Definition::InputObjectTypeExtension(n) => TypeDefinitionParts {
            input_fields: &n.fields,
            ..parts(&n.name, &n.directives)
        },
        Definition::EnumTypeDefinition(n) => TypeDefinitionParts {
            values: &n.values,
            ..parts(&n.name, &n.directives)
        },
        Definition::EnumTypeExtension(n) => TypeDefinitionParts {
            values: &n.values,
            ..parts(&n.name, &n.directives)
        },
        _ => return None,
    })
}

/// `collectTagsBySchemaCoordinateFromSubgraph(documentNode, map, subcoordinatesPerType)`.
/// Its nested functions (`addTypeFields`, `addTagsPerSchemaCoordinate`,
/// `TypeDefinitionHandler`, ...) become local helpers.
fn collect_tags_by_schema_coordinate_from_subgraph(
    document_node: &Document,
    map: &mut SchemaCoordinateToTagsRegistry,
    subcoordinates_per_type: &mut SubcoordinatesPerType,
) -> JsResult<()> {
    let link_implementations = extract_link_implementations(document_node)?;
    let tag_directive_name = link_implementations.resolve_import_name("https://specs.apollo.dev/federation", "@tag");
    let extract_tag = create_tag_directive_name_extraction_strategy(&tag_directive_name);

    fn add_type_fields(subcoordinates_per_type: &mut SubcoordinatesPerType, type_name: &str, fields: IndexSet<String>) {
        let type_fields = subcoordinates_per_type.entry(type_name.to_string()).or_default();
        for value in fields {
            type_fields.insert(value);
        }
    }

    fn add_tags_per_schema_coordinate(
        map: &mut SchemaCoordinateToTagsRegistry,
        schema_coordinate: &str,
        tag_values: Option<IndexSet<String>>,
    ) {
        let Some(tag_values) = tag_values else {
            return;
        };

        let values = map.entry(schema_coordinate.to_string()).or_default();
        for tag_value in tag_values {
            values.insert(tag_value);
        }
    }

    let get_tags_for_node = |directives: &[ConstDirective]| -> Option<IndexSet<String>> {
        let mut tags = IndexSet::default();
        for directive_node in directives {
            let Some(tag_value) = extract_tag(directive_node) else {
                continue;
            };
            tags.insert(tag_value);
        }
        if tags.is_empty() {
            return None;
        }
        Some(tags)
    };

    // `TypeDefinitionHandler` (returns `false`: nothing nested is visited).
    for definition in &document_node.definitions {
        let Some(node) = type_definition_parts(definition) else {
            continue;
        };
        let tag_values = get_tags_for_node(node.directives);
        add_tags_per_schema_coordinate(map, node.name, tag_values);

        let mut sub_coordinates: IndexSet<String> = IndexSet::default();

        // `node.fields?.forEach(...)`, with `"arguments" in fieldNode` for field definitions.
        for field_node in node.field_definitions {
            let schema_coordinate = format!("{}.{}", node.name, field_node.name.value);
            sub_coordinates.insert(schema_coordinate.clone());

            let tag_values = get_tags_for_node(&field_node.directives);
            add_tags_per_schema_coordinate(map, &schema_coordinate, tag_values);

            for argument_node in &field_node.arguments {
                let schema_coordinate = format!(
                    "{}.{}({}:)",
                    node.name, field_node.name.value, argument_node.name.value
                );
                sub_coordinates.insert(schema_coordinate.clone());

                let tag_values = get_tags_for_node(&argument_node.directives);
                add_tags_per_schema_coordinate(map, &schema_coordinate, tag_values);
            }
        }
        for field_node in node.input_fields {
            let schema_coordinate = format!("{}.{}", node.name, field_node.name.value);
            sub_coordinates.insert(schema_coordinate.clone());

            let tag_values = get_tags_for_node(&field_node.directives);
            add_tags_per_schema_coordinate(map, &schema_coordinate, tag_values);
        }
        for value_node in node.values {
            let schema_coordinate = format!("{}.{}", node.name, value_node.name.value);
            sub_coordinates.insert(schema_coordinate.clone());

            let tag_values = get_tags_for_node(&value_node.directives);
            add_tags_per_schema_coordinate(map, &schema_coordinate, tag_values);
        }

        add_type_fields(subcoordinates_per_type, node.name, sub_coordinates);
    }
    Ok(())
}

/// `SchemaCoordinateToTagsRegistry`: schema coordinate to tags.
pub type SchemaCoordinateToTagsRegistry = IndexMap<String, IndexSet<String>>;

/// `SubcoordinatesPerType`: type name to schema coordinates.
pub type SubcoordinatesPerType = IndexMap<String, IndexSet<String>>;

/// `buildSchemaCoordinateTagRegister(documentNodes)`: tags per schema coordinate in all
/// subgraphs (fields and arguments inherit their type's tags).
pub fn build_schema_coordinate_tag_register(document_nodes: &[&Document]) -> JsResult<SchemaCoordinateToTagsRegistry> {
    let mut schema_coordinates_to_tags: SchemaCoordinateToTagsRegistry = IndexMap::default();
    let mut subcoordinates_per_type: SubcoordinatesPerType = IndexMap::default();

    for document_node in document_nodes {
        collect_tags_by_schema_coordinate_from_subgraph(
            document_node,
            &mut schema_coordinates_to_tags,
            &mut subcoordinates_per_type,
        )?;
    }

    // The tags of a type are inherited by it's fields and field arguments
    for (type_name, sub_coordinates) in &subcoordinates_per_type {
        // NOTE(port): a copy of the type's tags; sub-coordinates contain a `.`, so they never
        // name a type and the copied set cannot change during this loop.
        let Some(tags) = schema_coordinates_to_tags.get(type_name).cloned() else {
            continue;
        };

        for sub_coordinate in sub_coordinates {
            let subcoordinate_tags = schema_coordinates_to_tags.entry(sub_coordinate.clone()).or_default();
            for tag in &tags {
                subcoordinate_tags.insert(tag.clone());
            }
        }
    }

    Ok(schema_coordinates_to_tags)
}

/// NOTE(port): typed `string | null` in TS but never `null`, so this is always `Some`.
fn get_root_query_type_name_from_document_node(document: &Document) -> Option<String> {
    let mut query_name: Option<String> = Some("Query".to_string());

    for definition in &document.definitions {
        let operation_types = match definition {
            Definition::SchemaDefinition(d) => &d.operation_types,
            Definition::SchemaExtension(d) => &d.operation_types,
            _ => continue,
        };
        for operation_type_definition in operation_types {
            if operation_type_definition.operation == OperationType::Query {
                query_name = Some(operation_type_definition.ty.name.value.clone());
            }
        }
    }

    query_name
}

/// `hasIntersection(a, b)`.
fn has_intersection<T: std::hash::Hash + Eq>(a: &IndexSet<T>, b: &IndexSet<T>) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a.iter().any(|item| b.contains(item))
}

/// `difference(set1, set2)`.
fn difference<T: std::hash::Hash + Eq + Clone>(set1: &IndexSet<T>, set2: &IndexSet<T>) -> IndexSet<T> {
    set1.iter().filter(|item| !set2.contains(*item)).cloned().collect()
}

/// The result of `applyTagFilterToInaccessibleTransformOnSubgraphSchema`.
#[derive(Clone, Debug)]
pub struct TagFilterTransformResult {
    pub type_defs: Document,
    /// Types within this subgraph where all fields are inaccessible.
    pub types_with_all_fields_inaccessible: IndexMap<String, bool>,
    pub transform_tag_directives: TransformTagDirectives,
    /// Types in this subgraph that have the inaccessible directive applied.
    pub types_with_inaccessible_applied: IndexSet<String>,
}

/// `(filter.include.size && !hasIntersection(tagsOnNode, filter.include)) ||
/// (filter.exclude.size && hasIntersection(tagsOnNode, filter.exclude))`.
fn is_excluded_by_filter(filter: &SchemaContractFilter, tags_on_node: &IndexSet<String>) -> bool {
    (!filter.include.is_empty() && !has_intersection(tags_on_node, &filter.include))
        || (!filter.exclude.is_empty() && has_intersection(tags_on_node, &filter.exclude))
}

/// `ObjectLikeNode`: object, interface and input object definitions and extensions.
fn is_object_like_node(definition: &Definition) -> bool {
    matches!(
        definition,
        Definition::ObjectTypeDefinition(_)
            | Definition::ObjectTypeExtension(_)
            | Definition::InterfaceTypeDefinition(_)
            | Definition::InterfaceTypeExtension(_)
            | Definition::InputObjectTypeDefinition(_)
            | Definition::InputObjectTypeExtension(_)
    )
}

/// `node.name.value` of a named definition.
fn definition_name(definition: &Definition) -> &str {
    definition
        .as_node_ref()
        .name()
        .map(|n| n.value.as_str())
        .unwrap_or_default()
}

/// `node.directives` of a type definition or extension (empty for other kinds).
fn definition_directives(definition: &Definition) -> &[ConstDirective] {
    match definition {
        Definition::ScalarTypeDefinition(n) => &n.directives,
        Definition::ObjectTypeDefinition(n) => &n.directives,
        Definition::InterfaceTypeDefinition(n) => &n.directives,
        Definition::UnionTypeDefinition(n) => &n.directives,
        Definition::EnumTypeDefinition(n) => &n.directives,
        Definition::InputObjectTypeDefinition(n) => &n.directives,
        Definition::ScalarTypeExtension(n) => &n.directives,
        Definition::ObjectTypeExtension(n) => &n.directives,
        Definition::InterfaceTypeExtension(n) => &n.directives,
        Definition::UnionTypeExtension(n) => &n.directives,
        Definition::EnumTypeExtension(n) => &n.directives,
        Definition::InputObjectTypeExtension(n) => &n.directives,
        _ => &[],
    }
}

fn definition_directives_mut(definition: &mut Definition) -> Option<&mut Vec<ConstDirective>> {
    Some(match definition {
        Definition::ScalarTypeDefinition(n) => &mut n.directives,
        Definition::ObjectTypeDefinition(n) => &mut n.directives,
        Definition::InterfaceTypeDefinition(n) => &mut n.directives,
        Definition::UnionTypeDefinition(n) => &mut n.directives,
        Definition::EnumTypeDefinition(n) => &mut n.directives,
        Definition::InputObjectTypeDefinition(n) => &mut n.directives,
        Definition::ScalarTypeExtension(n) => &mut n.directives,
        Definition::ObjectTypeExtension(n) => &mut n.directives,
        Definition::InterfaceTypeExtension(n) => &mut n.directives,
        Definition::UnionTypeExtension(n) => &mut n.directives,
        Definition::EnumTypeExtension(n) => &mut n.directives,
        Definition::InputObjectTypeExtension(n) => &mut n.directives,
        _ => return None,
    })
}

/// `applyTagFilterToInaccessibleTransformOnSubgraphSchema(documentNode, tagRegister, filter)`:
/// transforms the subgraph so that every field not included by the filter is `@inaccessible`.
pub fn apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
    document_node: &Document,
    tag_register: &SchemaCoordinateToTagsRegistry,
    filter: &SchemaContractFilter,
) -> JsResult<TagFilterTransformResult> {
    let link_implementations = extract_link_implementations(document_node)?;
    let inaccessible_directive_name =
        link_implementations.resolve_import_name("https://specs.apollo.dev/federation", "@inaccessible");
    let tag_directive_name = link_implementations.resolve_import_name("https://specs.apollo.dev/federation", "@tag");
    let external_directive_name =
        link_implementations.resolve_import_name("https://specs.apollo.dev/federation", "@external");

    let empty_tags: IndexSet<String> = IndexSet::default();
    let get_tags_for_schema_coordinate =
        |coordinate: &str| -> &IndexSet<String> { tag_register.get(coordinate).unwrap_or(&empty_tags) };

    let transform_tag_directives = create_transform_tag_directives(&tag_directive_name, &inaccessible_directive_name);
    let root_query_type_name = get_root_query_type_name_from_document_node(document_node);

    let mut types_with_all_fields_inaccessible_tracker: IndexMap<String, bool> = IndexMap::default();

    fn on_all_fields_inaccessible(tracker: &mut IndexMap<String, bool>, name: &str) {
        if !tracker.contains_key(name) {
            tracker.insert(name.to_string(), true);
        }
    }

    fn on_some_fields_accessible(tracker: &mut IndexMap<String, bool>, name: &str) {
        tracker.insert(name.to_string(), false);
    }

    let has_external = |directives: &[ConstDirective]| {
        directives
            .iter()
            .any(|d| d.name.value == external_directive_name)
    };

    let field_argument_handler =
        |object_like_name: &str, field_like_name: &str, node: &InputValueDefinition| -> InputValueDefinition {
            // Check for external tag because we cannot contribute directives to external fields.
            if has_external(&node.directives) {
                return node.clone();
            }

            let tags_on_node = get_tags_for_schema_coordinate(&format!(
                "{}.{}({}:)",
                object_like_name, field_like_name, node.name.value
            ));

            if is_excluded_by_filter(filter, tags_on_node) {
                return InputValueDefinition {
                    directives: transform_tag_directives.call(&node.directives, true),
                    ..node.clone()
                };
            }

            InputValueDefinition {
                directives: transform_tag_directives.call(&node.directives, false),
                ..node.clone()
            }
        };

    //
    // A type can be defined multiple times within a subgraph and we need to find all implementations
    // for determining whether the full type, or only some fields are part of the public contract schema
    // (the port keeps indexes into `document_node.definitions`).
    //
    let mut definitions_by_schema_coordinate: IndexMap<String, Vec<usize>> = IndexMap::default();
    for (index, definition) in document_node.definitions.iter().enumerate() {
        if is_object_like_node(definition) {
            definitions_by_schema_coordinate
                .entry(definition_name(definition).to_string())
                .or_default()
                .push(index);
        }
    }

    // Tracking for which type already has `@inaccessible` applied (can only occur once)
    let mut types_with_inaccessible_applied: IndexSet<String> = IndexSet::default();
    // These are later used within the visitor to actually replace the nodes (keyed by index).
    let mut replacement_type_nodes: IndexMap<usize, Definition> = IndexMap::default();

    for (type_name, nodes) in &definitions_by_schema_coordinate {
        // After processing all nodes implementing a type, we know whether all or only some fields are inaccessible
        let mut is_some_fields_accessible = false;

        for &index in nodes {
            let node = &document_node.definitions[index];
            let node_name = definition_name(node);
            let node_directives = definition_directives(node);
            let tags_on_node = get_tags_for_schema_coordinate(node_name);
            let node_is_external = has_external(node_directives);

            let mut new_node = node.clone();

            // `fields: node.fields?.map((fieldNode) => ...)` for field definitions.
            let mut map_field_definition = |field_node: &FieldDefinition| -> FieldDefinition {
                let tags_on_node = get_tags_for_schema_coordinate(&format!("{}.{}", node_name, field_node.name.value));

                let field_node = FieldDefinition {
                    arguments: field_node
                        .arguments
                        .iter()
                        .map(|argument_node| field_argument_handler(node_name, &field_node.name.value, argument_node))
                        .collect(),
                    ..field_node.clone()
                };

                // Check for external tag because we cannot contribute directives to external fields.
                if has_external(&field_node.directives) || node_is_external {
                    return field_node;
                }

                if is_excluded_by_filter(filter, tags_on_node) {
                    return FieldDefinition {
                        directives: transform_tag_directives.call(&field_node.directives, true),
                        ..field_node
                    };
                }

                is_some_fields_accessible = true;

                FieldDefinition {
                    directives: transform_tag_directives.call(&field_node.directives, false),
                    ..field_node
                }
            };
            match &mut new_node {
                Definition::ObjectTypeExtension(n) => {
                    n.fields = n.fields.iter().map(&mut map_field_definition).collect();
                }
                Definition::ObjectTypeDefinition(n) => {
                    n.fields = n.fields.iter().map(&mut map_field_definition).collect();
                }
                Definition::InterfaceTypeDefinition(n) => {
                    n.fields = n.fields.iter().map(&mut map_field_definition).collect();
                }
                Definition::InterfaceTypeExtension(n) => {
                    n.fields = n.fields.iter().map(&mut map_field_definition).collect();
                }
                _ => {}
            }

            // The same callback for input object fields (no `arguments`).
            let mut map_input_field = |field_node: &InputValueDefinition| -> InputValueDefinition {
                let tags_on_node = get_tags_for_schema_coordinate(&format!("{}.{}", node_name, field_node.name.value));

                // Check for external tag because we cannot contribute directives to external fields.
                if has_external(&field_node.directives) || node_is_external {
                    return field_node.clone();
                }

                if is_excluded_by_filter(filter, tags_on_node) {
                    return InputValueDefinition {
                        directives: transform_tag_directives.call(&field_node.directives, true),
                        ..field_node.clone()
                    };
                }

                is_some_fields_accessible = true;

                InputValueDefinition {
                    directives: transform_tag_directives.call(&field_node.directives, false),
                    ..field_node.clone()
                }
            };
            match &mut new_node {
                Definition::InputObjectTypeDefinition(n) => {
                    n.fields = n.fields.iter().map(&mut map_input_field).collect();
                }
                Definition::InputObjectTypeExtension(n) => {
                    n.fields = n.fields.iter().map(&mut map_input_field).collect();
                }
                _ => {}
            }

            let new_directives = if !filter.exclude.is_empty() && has_intersection(tags_on_node, &filter.exclude) {
                let directives = transform_tag_directives.call(
                    node_directives,
                    if types_with_inaccessible_applied.contains(type_name) { false } else { true },
                );
                types_with_inaccessible_applied.insert(type_name.clone());
                directives
            } else {
                transform_tag_directives.call(node_directives, false)
            };
            if let Some(directives) = definition_directives_mut(&mut new_node) {
                *directives = new_directives;
            }

            replacement_type_nodes.insert(index, new_node);
        }

        // If some fields are accessible, we continue with the next type
        if is_some_fields_accessible {
            on_some_fields_accessible(&mut types_with_all_fields_inaccessible_tracker, type_name);
            continue;
        }
        on_all_fields_inaccessible(&mut types_with_all_fields_inaccessible_tracker, type_name);
    }

    // `visit(documentNode, { ...: fieldLikeObjectHandler, ...: enumHandler, ...: scalarAndUnionHandler })`.
    let mut definitions: Vec<Definition> = Vec::with_capacity(document_node.definitions.len());
    for (index, node) in document_node.definitions.iter().enumerate() {
        let new_node = match node {
            // `fieldLikeObjectHandler`
            Definition::ObjectTypeDefinition(_)
            | Definition::ObjectTypeExtension(_)
            | Definition::InterfaceTypeDefinition(_)
            | Definition::InterfaceTypeExtension(_)
            | Definition::InputObjectTypeDefinition(_)
            | Definition::InputObjectTypeExtension(_) => match replacement_type_nodes.get(&index) {
                Some(new_node) => new_node.clone(),
                None => {
                    return Err(JsError::error(format!(
                        "Found type without transformation mapping. {} Name",
                        definition_name(node)
                    )));
                }
            },
            // `enumHandler`
            Definition::EnumTypeDefinition(_) | Definition::EnumTypeExtension(_) => {
                let node_name = definition_name(node);
                let node_directives = definition_directives(node);
                let tags_on_node = get_tags_for_schema_coordinate(node_name);

                let mut is_all_fields_inaccessible = true;

                let mut new_node = node.clone();
                let values = match &mut new_node {
                    Definition::EnumTypeDefinition(n) => &mut n.values,
                    Definition::EnumTypeExtension(n) => &mut n.values,
                    _ => unreachable!("matched an enum definition or extension"),
                };
                *values = values
                    .iter()
                    .map(|value_node| {
                        let tags_on_node =
                            get_tags_for_schema_coordinate(&format!("{}.{}", node_name, value_node.name.value));

                        if is_excluded_by_filter(filter, tags_on_node) {
                            return EnumValueDefinition {
                                directives: transform_tag_directives.call(&value_node.directives, true),
                                ..value_node.clone()
                            };
                        }

                        is_all_fields_inaccessible = false;

                        EnumValueDefinition {
                            directives: transform_tag_directives.call(&value_node.directives, false),
                            ..value_node.clone()
                        }
                    })
                    .collect();

                if !filter.exclude.is_empty() && has_intersection(tags_on_node, &filter.exclude) {
                    if let Some(directives) = definition_directives_mut(&mut new_node) {
                        *directives = transform_tag_directives.call(node_directives, true);
                    }
                } else {
                    if is_all_fields_inaccessible {
                        on_all_fields_inaccessible(&mut types_with_all_fields_inaccessible_tracker, node_name);
                    } else {
                        on_some_fields_accessible(&mut types_with_all_fields_inaccessible_tracker, node_name);
                    }

                    if let Some(directives) = definition_directives_mut(&mut new_node) {
                        *directives = transform_tag_directives.call(node_directives, false);
                    }
                }
                new_node
            }
            // `scalarAndUnionHandler` (definitions only; extensions are left as they are)
            Definition::ScalarTypeDefinition(_) | Definition::UnionTypeDefinition(_) => {
                let tags_on_node = get_tags_for_schema_coordinate(definition_name(node));
                let node_directives = definition_directives(node);

                let directives = if is_excluded_by_filter(filter, tags_on_node) {
                    transform_tag_directives.call(node_directives, true)
                } else {
                    transform_tag_directives.call(node_directives, false)
                };
                let mut new_node = node.clone();
                if let Some(d) = definition_directives_mut(&mut new_node) {
                    *d = directives;
                }
                new_node
            }
            _ => node.clone(),
        };
        definitions.push(new_node);
    }
    let type_defs = Document { definitions };

    if let Some(root_query_type_name) = &root_query_type_name {
        types_with_all_fields_inaccessible_tracker.shift_remove(root_query_type_name);
    }

    Ok(TagFilterTransformResult {
        type_defs,
        types_with_all_fields_inaccessible: types_with_all_fields_inaccessible_tracker,
        transform_tag_directives,
        types_with_inaccessible_applied,
    })
}

/// `applyTagFilterOnSubgraphs(subgraphs, filter)`: applies a tag filter to a set of subgraphs,
/// transforming `@tag` to `@inaccessible`.
/// NOTE(port): the TS is generic over `{ typeDefs }` and returns each subgraph spread with the
/// transform's extra properties (`typesWithAllFieldsInaccessible`, ...); here the subgraphs are
/// `ServiceDefinition`s and only `type_defs` changes. The bindings own any JS-visible extras.
pub fn apply_tag_filter_on_subgraphs(
    subgraphs: &[ServiceDefinition],
    filter: &SchemaContractFilter,
) -> JsResult<Vec<ServiceDefinition>> {
    let type_defs: Vec<&Document> = subgraphs.iter().map(|s| &s.type_defs).collect();
    Ok(apply_tag_filter_on_subgraph_type_defs(&type_defs, filter)?
        .into_iter()
        .zip(subgraphs)
        .map(|(result, subgraph)| ServiceDefinition {
            type_defs: result.type_defs,
            name: subgraph.name.clone(),
            url: subgraph.url.clone(),
        })
        .collect())
}

/// The body of `applyTagFilterOnSubgraphs` over the subgraphs' `typeDefs`: per subgraph, the
/// properties the TS spreads onto the returned subgraph object (`typeDefs`,
/// `typesWithAllFieldsInaccessible` after the cross-subgraph deletions, ...).
pub fn apply_tag_filter_on_subgraph_type_defs(
    type_defs: &[&Document],
    filter: &SchemaContractFilter,
) -> JsResult<Vec<TagFilterTransformResult>> {
    // All combined @tag directive in all subgraphs per schema coordinate
    let tag_register = build_schema_coordinate_tag_register(type_defs)?;

    let mut filtered_subgraphs: Vec<((), TagFilterTransformResult)> = Vec::with_capacity(type_defs.len());
    for type_defs in type_defs {
        let result = apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(type_defs, &tag_register, filter)?;
        filtered_subgraphs.push(((), result));
    }

    let mut intersection_of_types_where_all_fields_are_inaccessible: IndexSet<String> = IndexSet::default();
    // We need to traverse all subgraphs to find the intersection of types where all fields are inaccessible.
    // If a type is not present in any other subgraph, we can safely mark it as inaccessible.
    for index in 0..filtered_subgraphs.len() {
        // `otherSubgraphs` are all the others (identity comparison, so by index).
        // NOTE(port): the loop only deletes from the other subgraphs' maps, so this subgraph's
        // map does not change while it is iterated; iterating a snapshot is equivalent.
        let entries: Vec<(String, bool)> = filtered_subgraphs[index]
            .1
            .types_with_all_fields_inaccessible
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (r#type, all_fields_inaccessible) in entries {
            if all_fields_inaccessible
                && filtered_subgraphs
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index)
                    .all(|(_, (_, sub))| {
                        !sub.types_with_all_fields_inaccessible.contains_key(&r#type)
                            || sub.types_with_all_fields_inaccessible.get(&r#type) == Some(&true)
                    })
            {
                intersection_of_types_where_all_fields_are_inaccessible.insert(r#type.clone());
            }
            // let's not visit this type a second time...
            for (other, (_, sub)) in filtered_subgraphs.iter_mut().enumerate() {
                if other != index {
                    sub.types_with_all_fields_inaccessible.shift_remove(&r#type);
                }
            }
        }
    }

    if intersection_of_types_where_all_fields_are_inaccessible.is_empty() {
        return Ok(filtered_subgraphs.into_iter().map(|(_, result)| result).collect());
    }

    Ok(filtered_subgraphs
        .into_iter()
        .map(|(_, result)| TagFilterTransformResult {
            type_defs: make_types_from_set_inaccessible(
                &result.type_defs,
                // We exclude the types that are already marked as inaccessible within the subgraph as we want to avoid `@inaccessible` applied more than once.
                &difference(
                    &intersection_of_types_where_all_fields_are_inaccessible,
                    &result.types_with_inaccessible_applied,
                ),
                &result.transform_tag_directives,
            ),
            ..result
        })
        .collect())
}

/// `createTagDirectiveNameExtractionStrategy(directiveName)`.
/// NOTE(port): `directiveNode.arguments?.[0].name.value` throws a `TypeError` for an explicit
/// empty `arguments: []`, which graphql-js's parser never produces (it leaves `arguments`
/// undefined); the port cannot tell the two apart and treats both as undefined.
pub fn create_tag_directive_name_extraction_strategy(directive_name: &str) -> TagExtractionStrategy {
    let directive_name = directive_name.to_string();
    Rc::new(move |directive_node: &DirectiveNode| {
        if directive_node.name.value == directive_name {
            if let Some(first) = directive_node.arguments.first() {
                if first.name.value == "name" {
                    if let Value::String(value) = &first.value {
                        return Some(value.value.clone());
                    }
                }
            }
        }
        None
    })
}

/// `SchemaContractFilter`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchemaContractFilter {
    pub include: IndexSet<String>,
    pub exclude: IndexSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directives(sdl: &str) -> Vec<ConstDirective> {
        match graphql_js::parse(&format!("type T {sdl} {{ a: Int }}")).unwrap().definitions.remove(0) {
            Definition::ObjectTypeDefinition(n) => n.directives,
            _ => unreachable!(),
        }
    }

    fn names(directives: &[ConstDirective]) -> Vec<String> {
        directives.iter().map(|d| graphql_js::print(d)).collect()
    }

    #[test]
    fn transform_tag_directives_strips_tags_and_adds_inaccessible() {
        let transform = create_transform_tag_directives("tag", "inaccessible");
        let input = directives("@tag(name: \"a\") @key(fields: \"id\") @tag(name: \"b\")");
        assert_eq!(names(&transform.call(&input, false)), vec!["@key(fields: \"id\")"]);
        assert_eq!(
            names(&transform.call(&input, true)),
            vec!["@key(fields: \"id\")", "@inaccessible"]
        );
        let input = directives("@inaccessible @tag(name: \"a\")");
        assert_eq!(names(&transform.call(&input, true)), vec!["@inaccessible"]);
        assert!(transform.call(&[], false).is_empty());
    }

    #[test]
    fn tag_extraction_strategy() {
        let extract = create_tag_directive_name_extraction_strategy("tag");
        let input = directives(
            "@tag(name: \"a\") @tag(name: \"\") @tag(other: \"x\", name: \"b\") @tag(name: 1) @tag @foo(name: \"c\")",
        );
        let tags: Vec<Option<String>> = input.iter().map(|d| extract(d)).collect();
        assert_eq!(
            tags,
            vec![Some("a".into()), Some("".into()), None, None, None, None]
        );
    }

    #[test]
    fn set_helpers() {
        let a: IndexSet<String> = ["x", "y"].iter().map(|s| s.to_string()).collect();
        let b: IndexSet<String> = ["y"].iter().map(|s| s.to_string()).collect();
        let empty: IndexSet<String> = IndexSet::default();
        assert!(has_intersection(&a, &b));
        assert!(!has_intersection(&a, &empty));
        assert_eq!(difference(&a, &b).into_iter().collect::<Vec<_>>(), vec!["x".to_string()]);
    }

    #[test]
    fn root_query_type_name() {
        let doc = graphql_js::parse("type Query { a: Int }").unwrap();
        assert_eq!(get_root_query_type_name_from_document_node(&doc), Some("Query".into()));
        let doc = graphql_js::parse("schema { query: Root } extend schema { query: Other mutation: M }").unwrap();
        assert_eq!(get_root_query_type_name_from_document_node(&doc), Some("Other".into()));
    }

    #[test]
    fn make_types_inaccessible_once_per_name() {
        let doc = graphql_js::parse(
            "type A @tag(name: \"x\") { a: Int } extend type A { b: Int } enum E { V } type B { c: Int } union U = B",
        )
        .unwrap();
        let types: IndexSet<String> = ["A", "E", "U"].iter().map(|s| s.to_string()).collect();
        let result = make_types_from_set_inaccessible(&doc, &types, &create_transform_tag_directives("tag", "inaccessible"));
        assert_eq!(
            graphql_js::print(&result),
            "type A @inaccessible {\n  a: Int\n}\n\nextend type A {\n  b: Int\n}\n\nenum E @inaccessible {\n  V\n}\n\ntype B {\n  c: Int\n}\n\nunion U = B"
        );
    }
}
