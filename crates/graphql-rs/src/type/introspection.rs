//! Port of graphql-js `type/introspection`: the introspection types, as far
//! as schema construction and validation read them (names, fields, field
//! types, arguments and their defaults, enum values).
//!
//! NOTE(port): descriptions and resolvers are not carried; nothing that
//! builds or validates a schema reads them.

use crate::ast::Type;
use crate::collections::IndexMap;
use crate::directive_location::DirectiveLocation;
use crate::jsutils::JsValue;
use crate::parser::parse_type;

use super::definition::{
    DefaultInput, GraphQLArgumentConfig, GraphQLEnumType, GraphQLEnumTypeConfig, GraphQLEnumValueConfig,
    GraphQLFieldConfig, GraphQLNamedType, GraphQLObjectType, GraphQLObjectTypeConfig, GraphQLType, Lazy,
};

pub use crate::specified::is_introspection_type;

fn type_from_str(source: &str) -> GraphQLType {
    fn convert(node: &Type) -> GraphQLType {
        match node {
            Type::Named(named) => GraphQLType::named(named.name.value.clone()),
            Type::List(list) => GraphQLType::list(convert(&list.ty)),
            Type::NonNull(non_null) => GraphQLType::non_null(convert(&non_null.ty)),
        }
    }
    convert(&parse_type(source).expect("introspection field types parse"))
}

/// The `includeDeprecated: Boolean! = false` argument several introspection
/// fields take (`default: { value: false }`).
fn include_deprecated() -> IndexMap<String, GraphQLArgumentConfig> {
    let mut args = IndexMap::default();
    args.insert(
        "includeDeprecated".to_string(),
        GraphQLArgumentConfig {
            description: None,
            ty: type_from_str("Boolean!"),
            default: Some(DefaultInput::Value(JsValue::Boolean(false))),
            deprecation_reason: None,
            ast_node: None,
        },
    );
    args
}

fn object(name: &str, fields: Vec<(&str, &str, bool)>) -> GraphQLNamedType {
    let fields = fields
        .into_iter()
        .map(|(field_name, ty, with_include_deprecated)| {
            (
                field_name.to_string(),
                GraphQLFieldConfig {
                    description: None,
                    ty: type_from_str(ty),
                    args: if with_include_deprecated {
                        include_deprecated()
                    } else {
                        IndexMap::default()
                    },
                    deprecation_reason: None,
                    ast_node: None,
                },
            )
        })
        .collect();
    let object = GraphQLObjectType::new(GraphQLObjectTypeConfig {
        name: name.to_string(),
        description: None,
        interfaces: Lazy::new(Vec::new()),
        fields: Lazy::new(fields),
        ast_node: None,
        extension_ast_nodes: Vec::new(),
    })
    .expect("introspection type names are valid");
    object.get_fields().expect("introspection fields are valid");
    GraphQLNamedType::Object(object)
}

fn enumeration(name: &str, values: &[&str]) -> GraphQLNamedType {
    let values = values
        .iter()
        .map(|value| (value.to_string(), GraphQLEnumValueConfig::default()))
        .collect();
    let enum_type = GraphQLEnumType::new(GraphQLEnumTypeConfig {
        name: name.to_string(),
        description: None,
        values: Lazy::new(values),
        ast_node: None,
        extension_ast_nodes: Vec::new(),
    })
    .expect("introspection type names are valid");
    enum_type.values().expect("introspection enum values are valid");
    GraphQLNamedType::Enum(enum_type)
}

/// graphql-js `__Schema`.
pub fn schema_type() -> GraphQLNamedType {
    object(
        "__Schema",
        vec![
            ("description", "String", false),
            ("types", "[__Type!]!", false),
            ("queryType", "__Type!", false),
            ("mutationType", "__Type", false),
            ("subscriptionType", "__Type", false),
            ("directives", "[__Directive!]!", true),
        ],
    )
}

/// graphql-js `__Directive`.
pub fn directive_type() -> GraphQLNamedType {
    object(
        "__Directive",
        vec![
            ("name", "String!", false),
            ("description", "String", false),
            ("isRepeatable", "Boolean!", false),
            ("locations", "[__DirectiveLocation!]!", false),
            ("args", "[__InputValue!]!", true),
            ("isDeprecated", "Boolean!", false),
            ("deprecationReason", "String", false),
        ],
    )
}

/// graphql-js `__DirectiveLocation`.
pub fn directive_location_type() -> GraphQLNamedType {
    let values: Vec<&str> = DirectiveLocation::ALL.iter().map(|l| l.as_str()).collect();
    enumeration("__DirectiveLocation", &values)
}

/// graphql-js `__Type`.
pub fn type_type() -> GraphQLNamedType {
    object(
        "__Type",
        vec![
            ("kind", "__TypeKind!", false),
            ("name", "String", false),
            ("description", "String", false),
            ("specifiedByURL", "String", false),
            ("fields", "[__Field!]", true),
            ("interfaces", "[__Type!]", false),
            ("possibleTypes", "[__Type!]", false),
            ("enumValues", "[__EnumValue!]", true),
            ("inputFields", "[__InputValue!]", true),
            ("ofType", "__Type", false),
            ("isOneOf", "Boolean", false),
        ],
    )
}

/// graphql-js `__Field`.
pub fn field_type() -> GraphQLNamedType {
    object(
        "__Field",
        vec![
            ("name", "String!", false),
            ("description", "String", false),
            ("args", "[__InputValue!]!", true),
            ("type", "__Type!", false),
            ("isDeprecated", "Boolean!", false),
            ("deprecationReason", "String", false),
        ],
    )
}

/// graphql-js `__InputValue`.
pub fn input_value_type() -> GraphQLNamedType {
    object(
        "__InputValue",
        vec![
            ("name", "String!", false),
            ("description", "String", false),
            ("type", "__Type!", false),
            ("defaultValue", "String", false),
            ("isDeprecated", "Boolean!", false),
            ("deprecationReason", "String", false),
        ],
    )
}

/// graphql-js `__EnumValue`.
pub fn enum_value_type() -> GraphQLNamedType {
    object(
        "__EnumValue",
        vec![
            ("name", "String!", false),
            ("description", "String", false),
            ("isDeprecated", "Boolean!", false),
            ("deprecationReason", "String", false),
        ],
    )
}

/// graphql-js `__TypeKind`.
pub fn type_kind_type() -> GraphQLNamedType {
    enumeration(
        "__TypeKind",
        &[
            "SCALAR",
            "OBJECT",
            "INTERFACE",
            "UNION",
            "ENUM",
            "INPUT_OBJECT",
            "LIST",
            "NON_NULL",
        ],
    )
}

/// graphql-js `introspectionTypes`, in graphql-js order.
pub fn introspection_types() -> Vec<GraphQLNamedType> {
    vec![
        schema_type(),
        directive_type(),
        directive_location_type(),
        type_type(),
        field_type(),
        input_value_type(),
        enum_value_type(),
        type_kind_type(),
    ]
}
