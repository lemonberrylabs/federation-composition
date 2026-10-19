//! Port of `src/graphql/contains-supergraph-spec.ts`.

/// `containsSupergraphSpec(sdl)`: whether the SDL mentions any supergraph spec directive
/// (`@NAME`) or type (`[NAME` or whitespace + `NAME`).
/// NOTE(port): the TS builds one cached regex `\[NAME|\sNAME|@NAME|...` (names are not
/// escaped; they contain only `[A-Za-z_]`). JS `\s` includes Unicode whitespace.
pub fn contains_supergraph_spec(sdl: &str) -> bool {
    todo!()
}
