//! Port of graphql-js `utilities/typeComparators` (`isEqualType`,
//! `isTypeSubTypeOf`).

use crate::r#type::definition::GraphQLType;
use crate::r#type::schema::GraphQLSchema;

/// graphql-js `typeA === typeB`. Named types are one object per name; two
/// wrapping types are never the same object here, which gives the same
/// results: graphql-js recurses into wrapping types when identity fails.
fn is_same_type(type_a: &GraphQLType, type_b: &GraphQLType) -> bool {
    matches!((type_a, type_b), (GraphQLType::Named(a), GraphQLType::Named(b)) if a == b)
}

/// graphql-js `isEqualType`.
pub fn is_equal_type(type_a: &GraphQLType, type_b: &GraphQLType) -> bool {
    if is_same_type(type_a, type_b) {
        return true;
    }
    match (type_a, type_b) {
        (GraphQLType::NonNull(a), GraphQLType::NonNull(b)) => is_equal_type(a, b),
        (GraphQLType::List(a), GraphQLType::List(b)) => is_equal_type(a, b),
        _ => false,
    }
}

/// graphql-js `isTypeSubTypeOf`.
pub fn is_type_sub_type_of(
    schema: &GraphQLSchema,
    maybe_sub_type: &GraphQLType,
    super_type: &GraphQLType,
) -> bool {
    if is_same_type(maybe_sub_type, super_type) {
        return true;
    }
    if let GraphQLType::NonNull(super_of) = super_type {
        if let GraphQLType::NonNull(sub_of) = maybe_sub_type {
            return is_type_sub_type_of(schema, sub_of, super_of);
        }
        return false;
    }
    if let GraphQLType::NonNull(sub_of) = maybe_sub_type {
        return is_type_sub_type_of(schema, sub_of, super_type);
    }
    if let GraphQLType::List(super_of) = super_type {
        if let GraphQLType::List(sub_of) = maybe_sub_type {
            return is_type_sub_type_of(schema, sub_of, super_of);
        }
        return false;
    }
    if maybe_sub_type.is_list() {
        return false;
    }
    let (GraphQLType::Named(super_name), GraphQLType::Named(sub_name)) =
        (super_type, maybe_sub_type)
    else {
        return false;
    };
    let Some(super_named) = schema.get_type(super_name) else {
        return false;
    };
    super_named.is_abstract_type()
        && schema
            .get_type(sub_name)
            .is_some_and(|sub| sub.is_interface_type() || sub.is_object_type())
        && schema.is_sub_type(super_named, sub_name)
}
