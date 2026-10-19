//! Port of `src/graphql/supergraph-spec.ts`.

use std::sync::LazyLock;

use graphql_js::Kind;
use indexmap::IndexSet;

/// One entry of `SupergraphSpecNodes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupergraphSpecNode {
    pub name: String,
    pub kind: Kind,
}

/// Names of inputs, enums and scalars that are part of the supergraph spec,
/// but are not part of this composition library.
pub static EXTRA_FEDERATION_TYPE_NAMES: LazyLock<IndexSet<&'static str>> = LazyLock::new(|| {
    IndexSet::from([
        "_FieldSet",
        "join__DirectiveArguments", // from Federation v2 - connectors
    ])
});

/// Names of directives that are part of the supergraph spec,
/// but are not part of this composition library. Mostly from Federation v1 era.
pub static EXTRA_FEDERATION_DIRECTIVE_NAMES: LazyLock<IndexSet<&'static str>> = LazyLock::new(|| {
    IndexSet::from([
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
    todo!()
}
