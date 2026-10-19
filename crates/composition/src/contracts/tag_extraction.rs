//! Port of `src/contracts/tag-extraction.ts`.

use std::rc::Rc;

use graphql_js::{ConstDirective, Directive as DirectiveNode, Document};
use indexmap::{IndexMap, IndexSet};

use crate::types::ServiceDefinition;
use crate::utils::js::JsResult;

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
    /// `transformTagDirectives(node, includeInaccessibleDirective = false)`: the node's
    /// directives without `@tag`, plus `@inaccessible` if requested and missing.
    pub fn call(&self, directives: &[ConstDirective], include_inaccessible_directive: bool) -> Vec<ConstDirective> {
        todo!()
    }
}

/// `makeTypesFromSetInaccessible(documentNode, types, transformTagDirectives)`.
fn make_types_from_set_inaccessible(
    document_node: &Document,
    types: &IndexSet<String>,
    transform_tag_directives: &TransformTagDirectives,
) -> Document {
    todo!()
}

/// `collectTagsBySchemaCoordinateFromSubgraph(documentNode, map, subcoordinatesPerType)`.
/// Its nested functions (`addTypeFields`, `addTagsPerSchemaCoordinate`,
/// `TypeDefinitionHandler`, ...) become local helpers.
fn collect_tags_by_schema_coordinate_from_subgraph(
    document_node: &Document,
    map: &mut SchemaCoordinateToTagsRegistry,
    subcoordinates_per_type: &mut SubcoordinatesPerType,
) -> JsResult<()> {
    todo!()
}

/// `SchemaCoordinateToTagsRegistry`: schema coordinate to tags.
pub type SchemaCoordinateToTagsRegistry = IndexMap<String, IndexSet<String>>;

/// `SubcoordinatesPerType`: type name to schema coordinates.
pub type SubcoordinatesPerType = IndexMap<String, IndexSet<String>>;

/// `buildSchemaCoordinateTagRegister(documentNodes)`: tags per schema coordinate in all
/// subgraphs (fields and arguments inherit their type's tags).
pub fn build_schema_coordinate_tag_register(document_nodes: &[&Document]) -> JsResult<SchemaCoordinateToTagsRegistry> {
    todo!()
}

fn get_root_query_type_name_from_document_node(document: &Document) -> Option<String> {
    todo!()
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

/// `applyTagFilterToInaccessibleTransformOnSubgraphSchema(documentNode, tagRegister, filter)`:
/// transforms the subgraph so that every field not included by the filter is `@inaccessible`.
pub fn apply_tag_filter_to_inaccessible_transform_on_subgraph_schema(
    document_node: &Document,
    tag_register: &SchemaCoordinateToTagsRegistry,
    filter: &SchemaContractFilter,
) -> JsResult<TagFilterTransformResult> {
    todo!()
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
    todo!()
}

/// `createTagDirectiveNameExtractionStrategy(directiveName)`.
pub fn create_tag_directive_name_extraction_strategy(directive_name: &str) -> TagExtractionStrategy {
    todo!()
}

/// `SchemaContractFilter`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchemaContractFilter {
    pub include: IndexSet<String>,
    pub exclude: IndexSet<String>,
}
