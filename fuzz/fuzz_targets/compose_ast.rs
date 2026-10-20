//! `federation_composition_rs::compose_services` on subgraph documents that the parser cannot
//! produce, as a document built in code or deserialized (a JavaScript AST through the
//! WebAssembly bindings) can be: names that are not GraphQL names.
//!
//! The input is `subgraphs \x01 names`: the subgraphs as in the `compose` target (split at NUL
//! bytes), and names separated by `\x02`. The `k`-th name node of the documents, in traversal
//! order, is renamed to the `k`-th name when that one is not empty.
#![no_main]

use federation_composition_rs::graphql_rs::{
    parse, visit_edit, EditAction, EditVisitor, Name, Node, NodeRef, VisitInfo,
};
use federation_composition_rs::{compose_services, ServiceDefinition};
use libfuzzer_sys::fuzz_target;

struct Rename<'n> {
    names: &'n [&'n str],
    next: usize,
}

impl EditVisitor for Rename<'_> {
    fn enter(&mut self, node: NodeRef<'_>, _: &VisitInfo<'_, '_>) -> EditAction {
        let NodeRef::Name(_) = node else {
            return EditAction::Continue;
        };
        let name = self.names.get(self.next).copied().unwrap_or("");
        self.next += 1;
        if name.is_empty() {
            EditAction::Continue
        } else {
            EditAction::Replace(Node::Name(Name::new(name)))
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let (subgraphs, names) = text.split_once('\x01').unwrap_or((text, ""));
    let names: Vec<&str> = names.split('\x02').collect();
    let mut rename = Rename { names: &names, next: 0 };
    let services: Vec<ServiceDefinition> = subgraphs
        .split('\0')
        .take(4)
        .zip(["a", "b", "c", "d"])
        .filter_map(|(sdl, name)| {
            let document = parse(sdl).ok()?;
            Some(ServiceDefinition::new(
                name,
                visit_edit(&document, &mut rename).into_document(),
            ))
        })
        .collect();
    if services.is_empty() {
        return;
    }
    let Ok(result) = compose_services(&services) else {
        return;
    };
    if let Some(success) = result.as_success() {
        let _ = success.public_sdl();
    }
});
