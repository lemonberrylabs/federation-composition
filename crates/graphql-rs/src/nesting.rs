//! Not in graphql-js: the nesting depth of an AST that was not produced by this crate's parser
//! (built in code or deserialized), measured without recursion so that it is safe on any input.

use crate::ast::{NodeRef, SlotRef, Type};
use crate::error::GraphQLError;

/// Checks that `node` nests no deeper than `max_depth` levels, counted the way the parser counts
/// them for [`ParseOptions::max_nesting_depth`](crate::ParseOptions::max_nesting_depth): each list
/// value, object value, selection set and list type is one level. A non-null type directly inside
/// another non-null type, which the parser cannot produce, is one level too, so that every chain
/// of nested nodes is counted.
///
/// A document the parser accepted (with the same limit) always passes. Use this on documents built
/// in code or deserialized before passing them to [`print`](crate::print), [`visit`](crate::visit)
/// or [`build_ast_schema`](crate::build_ast_schema), which recurse once per level. The functions of
/// this repository's crates that take a document and return a `Result` run this check themselves
/// with [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH).
///
/// The `Err` message is `AST nesting depth exceeds {max_depth}.`
///
/// ```
/// use graphql_rs::{check_nesting_depth, parse, MAX_NESTING_DEPTH};
///
/// let document = parse("{ a { b } }")?;
/// assert!(check_nesting_depth(&document, MAX_NESTING_DEPTH).is_ok());
/// assert!(check_nesting_depth(&document, 1).is_err());
/// # Ok::<(), graphql_rs::GraphQLError>(())
/// ```
pub fn check_nesting_depth<'a>(node: impl Into<NodeRef<'a>>, max_depth: usize) -> Result<(), GraphQLError> {
    let mut stack: Vec<(NodeRef<'a>, usize)> = vec![(node.into(), 0)];
    while let Some((node, enclosing)) = stack.pop() {
        let depth = enclosing + usize::from(is_level(node));
        if depth > max_depth {
            return Err(GraphQLError::new(format!("AST nesting depth exceeds {max_depth}.")));
        }
        for &(_, slot) in node.children().as_slice() {
            match slot {
                SlotRef::Absent => {}
                SlotRef::One(child) => stack.push((child, depth)),
                SlotRef::Many(list) => stack.extend(list.iter().map(|child| (child, depth))),
            }
        }
    }
    Ok(())
}

/// Whether `node` is one nesting level.
fn is_level(node: NodeRef<'_>) -> bool {
    match node {
        NodeRef::ListValue(_) | NodeRef::ObjectValue(_) | NodeRef::SelectionSet(_) | NodeRef::ListType(_) => true,
        NodeRef::NonNullType(n) => matches!(*n.ty, Type::NonNull(_)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ListType, NamedType, NonNullType};
    use crate::parser::{parse, MAX_NESTING_DEPTH};

    #[test]
    fn parsed_documents_at_the_limit_pass() {
        let n = MAX_NESTING_DEPTH;
        for source in [
            format!("{{ {}a{} }}", "a { ".repeat(n - 1), " }".repeat(n - 1)),
            format!("type Q {{ f(a: {}Int{}): Int }}", "[".repeat(n), "!]".repeat(n)),
            format!(
                "type Q {{ f(a: Int = {}1{}): Int }}",
                "[{a: ".repeat(n / 2),
                "}]".repeat(n / 2)
            ),
        ] {
            let document = parse(&source).expect("within the limit");
            check_nesting_depth(&document, MAX_NESTING_DEPTH).expect("within the limit");
            assert!(check_nesting_depth(&document, MAX_NESTING_DEPTH - 1).is_err());
        }
    }

    #[test]
    fn nested_non_null_types_are_levels() {
        let mut ty = Type::Named(NamedType::new("Int"));
        for _ in 0..1_000_000 {
            ty = Type::NonNull(NonNullType { ty: Box::new(ty) });
        }
        let err = check_nesting_depth(&ty, MAX_NESTING_DEPTH).unwrap_err();
        assert_eq!(err.message, "AST nesting depth exceeds 128.");
        let list = Type::List(ListType {
            ty: Box::new(Type::NonNull(NonNullType {
                ty: Box::new(Type::Named(NamedType::new("Int"))),
            })),
        });
        check_nesting_depth(&list, 1).expect("one level");
        // Dropping a million nested boxes recurses; leak it instead.
        std::mem::forget(ty);
    }
}
