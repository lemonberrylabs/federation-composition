//! `federation_composition::compose_services` on up to four subgraphs: the input is split at NUL
//! bytes and each part that parses is a subgraph (`a`, `b`, ...). A successful composition is
//! also turned into its public schema, sorted, and composed again as a schema contract.
#![no_main]

use federation_composition::graphql_core::parse;
use federation_composition::{
    compose_schema_contract, compose_services, extract_link_implementations, sort_sdl,
    transform_supergraph_to_public_schema, validate_subgraph, SchemaContractFilter, ServiceDefinition,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let services: Vec<ServiceDefinition> = text
        .split('\0')
        .take(4)
        .zip(["a", "b", "c", "d"])
        .filter_map(|(sdl, name)| Some(ServiceDefinition::new(name, parse(sdl).ok()?)))
        .collect();
    if services.is_empty() {
        return;
    }
    for service in &services {
        let _ = extract_link_implementations(&service.type_defs);
    }
    if services.len() == 1 {
        let _ = validate_subgraph(&services[0]);
    }
    let Ok(result) = compose_services(&services) else {
        return;
    };
    let Some(success) = result.as_success() else {
        return;
    };
    let _ = success.public_sdl();
    let supergraph = success.supergraph_document_node();
    let _ = sort_sdl(supergraph);
    let _ = transform_supergraph_to_public_schema(supergraph);
    let filter = SchemaContractFilter::new(["public"], ["internal"]);
    let _ = compose_schema_contract(&services, &filter, true);
});
