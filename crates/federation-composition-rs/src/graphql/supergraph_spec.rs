//! Port of `src/graphql/supergraph-spec.ts`.

use std::sync::LazyLock;

use graphql_rs::{is_type_definition_node, parse, Definition, Kind};
use crate::utils::collections::IndexSet;

use crate::specifications::federation::get_latest_federation_version;
use crate::specifications::{authenticated, context, cost, inaccessible, join, link, policy, requires_scopes, tag};

/// One entry of `SupergraphSpecNodes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupergraphSpecNode {
    pub name: String,
    pub kind: Kind,
}

/// Names of inputs, enums and scalars that are part of the supergraph spec,
/// but are not part of this composition library.
pub static EXTRA_FEDERATION_TYPE_NAMES: LazyLock<IndexSet<&'static str>> = LazyLock::new(|| {
    IndexSet::from_iter([
        "_FieldSet",
        "join__DirectiveArguments", // from Federation v2 - connectors
    ])
});

/// Names of directives that are part of the supergraph spec,
/// but are not part of this composition library. Mostly from Federation v1 era.
pub static EXTRA_FEDERATION_DIRECTIVE_NAMES: LazyLock<IndexSet<&'static str>> = LazyLock::new(|| {
    IndexSet::from_iter([
        "core",            // from Federation v1 supergraph
        "join__owner",     // from Federation v1 supergraph
        "join__directive", // from Federation v2 - connectors
        "context",         // from Federation v2.8 - we don't support it yet
    ])
});

/// The cached `supergraphSpecNodes`.
static SUPERGRAPH_SPEC_NODES: LazyLock<Vec<SupergraphSpecNode>> = LazyLock::new(compute_supergraph_spec_nodes);

/// `getSupergraphSpecNodes()`.
pub fn get_supergraph_spec_nodes() -> &'static [SupergraphSpecNode] {
    &SUPERGRAPH_SPEC_NODES
}

/// The body of `getSupergraphSpecNodes` that runs on the first call.
fn compute_supergraph_spec_nodes() -> Vec<SupergraphSpecNode> {
    let latest_version = get_latest_federation_version();

    [
        authenticated::SDL.to_string(),
        context::SDL.to_string(),
        cost::sdl(&cost::CostNames {
            cost: "cost".to_string(),
            list_size: "listSize".to_string(),
        }),
        inaccessible::SDL.to_string(),
        join::sdl(latest_version),
        link::SDL.to_string(),
        policy::SDL.to_string(),
        requires_scopes::SDL.to_string(),
        tag::SDL.to_string(),
    ]
    .iter()
    .flat_map(|sdl| {
        parse(sdl)
            .expect("valid supergraph spec SDL")
            .definitions
            .into_iter()
            .filter_map(|d| match &d {
                Definition::DirectiveDefinition(x) => Some(SupergraphSpecNode {
                    name: x.name.value.clone(),
                    kind: d.kind(),
                }),
                _ if is_type_definition_node(&d) => d.name().map(|name| SupergraphSpecNode {
                    name: name.value.clone(),
                    kind: d.kind(),
                }),
                _ => None,
            })
            .collect::<Vec<_>>()
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_nodes() {
        let nodes = get_supergraph_spec_nodes();
        let names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(&names[..4], ["authenticated", "context", "context__fromContext", "context__ContextFieldValue"]);
        assert!(names.contains(&"join__ContextArgument"));
        assert!(names.contains(&"join__FieldValue"));
        assert_eq!(names.last(), Some(&"tag"));
        assert_eq!(nodes[0].kind, Kind::DirectiveDefinition);
        assert_eq!(nodes[3].kind, Kind::ScalarTypeDefinition);
    }
}
