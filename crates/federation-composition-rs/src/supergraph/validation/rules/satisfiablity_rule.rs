//! Port of `src/supergraph/validation/rules/satisfiablity-rule.ts`.

use std::rc::Rc;

use graphql_rs::{is_specified_scalar_type, print, specified_scalar_types, GraphQLError, ObjectField, Value};
use crate::utils::collections::IndexMap;

use super::satisfiablity::edge::is_field_edge;
use super::satisfiablity::errors::SatisfiabilityError;
use super::satisfiablity::graph::Graph;
use super::satisfiablity::moves::Move;
use super::satisfiablity::operation_path::Step;
use super::satisfiablity::supergraph::Supergraph;
use super::satisfiablity::walker::WalkTracker;
use crate::supergraph::composition::interface_type::{InterfaceTypeFieldState, InterfaceTypeState};
use crate::supergraph::composition::object_type::{ObjectTypeFieldState, ObjectTypeState};
use crate::supergraph::composition::visitor::SupergraphVisitor;
use crate::supergraph::state::SupergraphState;
use crate::supergraph::validation::validation_context::{SupergraphRule, SupergraphValidationContext};
use crate::utils::js::{JsError, JsResult};
use crate::utils::state::{is_list, is_non_null, strip_non_null, strip_type_modifiers};

/// `SatisfiabilityRule(context, supergraph)`. `new` builds the satisfiability `Supergraph` and runs `validate()`; fields: `error_by_coordinate` (`Record<string, WalkTracker[]>`; coordinates are type/field names, never integer-like).
/// Visitor callbacks: `InterfaceType`, `ObjectType`, `InterfaceTypeField`, `ObjectTypeField`.
/// Fields: the locals of the TS factory (and `supergraph` if the callbacks read it).
pub struct SatisfiabilityRule<'s> {
    supergraph: &'s SupergraphState,
    /// The satisfiability supergraph graph: resolves the edges of each tracker's `superPath`.
    supergraph_graph: Rc<Graph<'s>>,
    error_by_coordinate: IndexMap<String, Vec<WalkTracker<'s>>>,
}

impl<'s> SupergraphRule<'s> for SatisfiabilityRule<'s> {
    const NAME: &'static str = "SatisfiabilityRule";

    fn new(_context: &mut SupergraphValidationContext, supergraph: &'s SupergraphState) -> JsResult<Self> {
        let satisfiability_supergraph = Supergraph::new(supergraph)?;

        let unreachables = satisfiability_supergraph.validate()?;
        let supergraph_graph = satisfiability_supergraph.supergraph_graph().clone();

        // NOTE(port): a plain object in TS. Lookups of inherited keys (a type named `constructor`
        // or `toString`) would hit `Object.prototype` there; such type names are not handled.
        let mut error_by_coordinate: IndexMap<String, Vec<WalkTracker<'s>>> = IndexMap::default();

        for unreachable in unreachables {
            let Some(edge) = unreachable.super_path.edge() else {
                return Err(JsError::error("Expected edge to be defined"));
            };

            let e = supergraph_graph.edge(edge);
            let coordinate = match &e.r#move {
                Move::Field(field_move) if is_field_edge(e) => {
                    format!("{}.{}", field_move.type_name, field_move.field_name)
                }
                _ => supergraph_graph.node(e.head).type_name.clone(),
            };

            error_by_coordinate.entry(coordinate).or_default().push(unreachable);
        }

        Ok(SatisfiabilityRule {
            supergraph,
            supergraph_graph,
            error_by_coordinate,
        })
    }
}

/// The keys of a JS object in `for..in` order: integer-like keys (array indexes) first in
/// ascending numeric order, then the other keys in insertion order.
fn js_object_key_order<'a>(keys: impl IntoIterator<Item = &'a String>) -> Vec<&'a String> {
    fn array_index(key: &str) -> Option<u32> {
        if key.is_empty() || (key.len() > 1 && key.starts_with('0')) || !key.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // Array indexes are 0 ..= 2^32 - 2.
        key.parse::<u64>().ok().filter(|&n| n < u32::MAX as u64).map(|n| n as u32)
    }

    let mut integer_keys: Vec<(u32, &String)> = Vec::new();
    let mut other_keys: Vec<&String> = Vec::new();
    for key in keys {
        match array_index(key) {
            Some(n) => integer_keys.push((n, key)),
            None => other_keys.push(key),
        }
    }
    integer_keys.sort_by_key(|(n, _)| *n);
    integer_keys.into_iter().map(|(_, k)| k).chain(other_keys).collect()
}

impl<'s> SatisfiabilityRule<'s> {
    /// `check(typeName, fieldName?)`: reports the unreachable paths recorded for the coordinate.
    fn check(&mut self, cx: &mut SupergraphValidationContext, type_name: &str, field_name: Option<&str>) -> JsResult<()> {
        // PERF: runs for every type and field; when nothing is unreachable the lookup below
        // always misses, so skip building the coordinate.
        if self.error_by_coordinate.is_empty() {
            return Ok(());
        }

        let coordinate = match field_name {
            Some(field_name) if !field_name.is_empty() => format!("{type_name}.{field_name}"),
            _ => type_name.to_string(),
        };

        let Some(unreachables) = self.error_by_coordinate.get(&coordinate) else {
            return Ok(());
        };

        if unreachables.is_empty() {
            return Ok(());
        }

        for unreachable in unreachables {
            let query_path: QueryPath = unreachable
                .super_path
                .steps(&self.supergraph_graph)
                .into_iter()
                .map(|step| match step {
                    Step::Field { field_name, type_name } => QueryPathEntry {
                        type_name,
                        field_name: Some(field_name),
                    },
                    Step::Abstract { type_name } => QueryPathEntry {
                        type_name,
                        field_name: None,
                    },
                })
                .collect();
            let query_string = print_query_path(self.supergraph, &query_path)?;

            if query_string.is_empty() {
                return Ok(());
            }

            // A plain object in TS (`Record<string, SatisfiabilityError[]>`) iterated with `for..in`.
            let mut errors_by_source_graph: IndexMap<String, Vec<SatisfiabilityError>> = IndexMap::default();
            let mut reasons: Vec<(String, Vec<String>)> = Vec::new();

            for error in unreachable.list_errors(&self.supergraph_graph)? {
                errors_by_source_graph
                    .entry(error.source_graph_name.clone())
                    .or_default()
                    .push(error);
            }

            // NOTE(port): subgraph names could be integer-like, so the JS key order is applied.
            for source_graph_name in js_object_key_order(errors_by_source_graph.keys()) {
                let errors = &errors_by_source_graph[source_graph_name];
                reasons.push((source_graph_name.clone(), errors.iter().map(|e| e.message.clone()).collect()));
            }

            if reasons.is_empty() {
                continue;
            }

            let mut lines: Vec<String> = vec![
                "The following supergraph API query:".to_string(),
                query_string,
                "cannot be satisfied by the subgraphs because:".to_string(),
            ];
            for (graph_name, reasons) in &reasons {
                if reasons.len() == 1 {
                    lines.push(format!("- from subgraph \"{graph_name}\": {}", reasons[0]));
                } else {
                    lines.push(format!(
                        "- from subgraph \"{graph_name}\":\n{}",
                        reasons.iter().map(|r| format!("  - {r}")).collect::<Vec<_>>().join("\n")
                    ));
                }
            }

            cx.report_error(GraphQLError::with_code(lines.join("\n"), "SATISFIABILITY_ERROR"));
        }

        Ok(())
    }
}

impl<'s> SupergraphVisitor<'s> for SatisfiabilityRule<'s> {
    fn interface_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
    ) -> JsResult<()> {
        self.check(cx, &interface_state.name, None)?;
        for type_name in &interface_state.implemented_by {
            self.check(cx, type_name, None)?;
        }
        Ok(())
    }

    fn object_type(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
    ) -> JsResult<()> {
        self.check(cx, &object_state.name, None)
    }

    fn interface_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        interface_state: &'s InterfaceTypeState,
        field_state: &'s InterfaceTypeFieldState,
    ) -> JsResult<()> {
        self.check(cx, &interface_state.name, Some(&field_state.name))?;
        for type_name in &interface_state.implemented_by {
            self.check(cx, type_name, Some(&field_state.name))?;
        }
        Ok(())
    }

    fn object_type_field(
        &mut self,
        cx: &mut SupergraphValidationContext,
        object_state: &'s ObjectTypeState,
        field_state: &'s ObjectTypeFieldState,
    ) -> JsResult<()> {
        self.check(cx, &object_state.name, Some(&field_state.name))
    }
}

/// An entry of `QueryPath`: `{ typeName, fieldName }` or `{ typeName }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPathEntry {
    pub type_name: String,
    pub field_name: Option<String>,
}

/// `QueryPath`.
pub type QueryPath = Vec<QueryPathEntry>;

fn print_line(msg: &str, indent_level: usize) -> String {
    "  ".repeat(indent_level + 1) + msg
}

/// What `printQueryPath` reads from an object or interface field state.
struct PrintedField<'a> {
    r#type: &'a str,
    graph_count: usize,
    /// `[name, type, hasFromContext, graphCount]` of each argument.
    args: Vec<(&'a str, &'a str, bool, usize)>,
}

impl<'a> PrintedField<'a> {
    fn of_object(f: &'a ObjectTypeFieldState) -> Self {
        PrintedField {
            r#type: &f.r#type,
            graph_count: f.by_graph.len(),
            args: f
                .args
                .iter()
                .map(|(name, a)| (name.as_str(), a.r#type.as_str(), a.from_context.is_some(), a.by_graph.len()))
                .collect(),
        }
    }

    fn of_interface(f: &'a InterfaceTypeFieldState) -> Self {
        PrintedField {
            r#type: &f.r#type,
            graph_count: f.by_graph.len(),
            args: f
                .args
                .iter()
                .map(|(name, a)| (name.as_str(), a.r#type.as_str(), a.from_context.is_some(), a.by_graph.len()))
                .collect(),
        }
    }
}

/// `printQueryPath(supergraphState, queryPath)`: the example operation in the error message.
fn print_query_path(supergraph_state: &SupergraphState, query_path: &QueryPath) -> JsResult<String> {
    let mut lines: Vec<String> = Vec::new();

    let mut ends_with_scalar = false;

    for (i, point) in query_path.iter().enumerate() {
        if let Some(field_name) = &point.field_name {
            let Some(type_state) = supergraph_state.object_types.get(&point.type_name) else {
                return Err(JsError::error(format!(
                    "Object type \"{}\" not found in Supergraph state",
                    point.type_name
                )));
            };

            let mut field_state = type_state.fields.get(field_name).map(PrintedField::of_object);

            if field_state.is_none() {
                for interface_name in &type_state.interfaces {
                    let Some(interface_state) = supergraph_state.interface_types.get(interface_name) else {
                        return Err(JsError::error(format!(
                            "Interface type \"{interface_name}\" not found in Supergraph state"
                        )));
                    };

                    field_state = interface_state.fields.get(field_name).map(PrintedField::of_interface);

                    if field_state.is_some() {
                        break;
                    }
                }
            }

            let Some(field_state) = field_state else {
                return Err(JsError::error(format!(
                    "Field \"{}.{field_name}\" not found in Supergraph state",
                    point.type_name
                )));
            };

            // Print the arguments as they appear in the public schema.
            // A `@fromContext` argument is resolved by the router, it's not part of the query.
            let mut printed_args: Vec<String> = Vec::new();
            for &(name, arg_type, has_from_context, arg_graph_count) in &field_state.args {
                if has_from_context || arg_graph_count != field_state.graph_count {
                    continue;
                }
                printed_args.push(format!(
                    "{name}: {}",
                    print(&create_empty_value_node(arg_type, supergraph_state)?)
                ));
            }
            let args = printed_args.join(", ");
            let args_printed = if !args.is_empty() { format!("({args})") } else { String::new() };

            if i == query_path.len() - 1 {
                let output_type_name = strip_type_modifiers(field_state.r#type);
                ends_with_scalar = supergraph_state.scalar_types.contains_key(&output_type_name)
                    || supergraph_state.enum_types.contains_key(&output_type_name)
                    || is_specified_scalar_type(&output_type_name);

                if ends_with_scalar {
                    lines.push(print_line(&format!("{field_name}{args_printed}"), i));
                } else {
                    lines.push(print_line(&format!("{field_name}{args_printed} {{"), i));
                }
            } else {
                lines.push(print_line(&format!("{field_name}{args_printed} {{"), i));
            }
        } else {
            lines.push(print_line(&format!("... on {} {{", point.type_name), i));
        }
    }

    if !ends_with_scalar {
        let indent = lines.len();
        lines.push(print_line("...", indent));
    }

    // `lines.length - 1` is at least 0: there is always a line.
    let len = lines.len().saturating_sub(1);
    for i in 0..len {
        lines.push(print_line("}", len - i - 1));
    }

    let Some(first) = query_path.first() else {
        return Err(JsError::type_error("Cannot read properties of undefined (reading 'typeName')"));
    };
    if first.type_name == "Query" {
        lines.insert(0, "{".to_string());
    } else if first.type_name == "Mutation" {
        lines.insert(0, "mutation {".to_string());
    } else {
        lines.insert(0, "subscription {".to_string());
    }
    lines.push("}".to_string());

    Ok(lines.join("\n"))
}

/// `createEmptyValueNode(fullType, supergraphState)`. Throws for unknown/unsupported types.
fn create_empty_value_node(full_type: &str, supergraph_state: &SupergraphState) -> JsResult<Value> {
    if is_list(full_type) {
        return Ok(Value::list(Vec::new()));
    }

    if is_non_null(full_type) {
        let inner_type = strip_non_null(full_type);

        return create_empty_value_node(&inner_type, supergraph_state);
    }

    if let Some(enum_state) = supergraph_state.enum_types.get(full_type) {
        // NOTE(port): an enum without values gives `undefined` in TS; composition rejects
        // empty enums before this rule runs.
        return Ok(Value::enum_value(enum_state.values.keys().next().cloned().unwrap_or_default()));
    }

    if supergraph_state.scalar_types.contains_key(full_type) {
        return Ok(Value::string("A string value"));
    }

    if let Some(input_object_type_state) = supergraph_state.input_object_types.get(full_type) {
        let mut fields = Vec::new();
        for (field_name, field_state) in &input_object_type_state.fields {
            if !is_non_null(&field_state.r#type) {
                continue;
            }
            fields.push(ObjectField::new(
                field_name.clone(),
                create_empty_value_node(&field_state.r#type, supergraph_state)?,
            ));
        }
        return Ok(Value::object(fields));
    }

    let Some(specified_scalar) = specified_scalar_types().iter().find(|s| s.name == full_type) else {
        return Err(JsError::error(format!("Type \"{full_type}\" is not defined.")));
    };

    if specified_scalar.name == "String" {
        return Ok(Value::string("A string value"));
    }

    if specified_scalar.name == "Int" || specified_scalar.name == "Float" {
        return Ok(Value::int("0"));
    }

    if specified_scalar.name == "Boolean" {
        return Ok(Value::boolean(true));
    }

    if specified_scalar.name == "ID" {
        return Ok(Value::string("<any id>"));
    }

    Err(JsError::error(format!("Type \"{full_type}\" is not supported.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_line_indents() {
        assert_eq!(print_line("a", 0), "  a");
        assert_eq!(print_line("}", 2), "      }");
    }

    #[test]
    fn js_object_key_order_puts_array_indexes_first() {
        let keys: Vec<String> = ["b", "10", "a", "2", "01", "4294967295", "4294967294", "-1", "0"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let ordered: Vec<&str> = js_object_key_order(&keys).into_iter().map(|s| s.as_str()).collect();
        assert_eq!(
            ordered,
            vec!["0", "2", "10", "4294967294", "b", "a", "01", "4294967295", "-1"]
        );
    }

    #[test]
    fn empty_values_for_builtin_types() {
        let state = SupergraphState::default();
        assert_eq!(print(&create_empty_value_node("String!", &state).unwrap()), "\"A string value\"");
        assert_eq!(print(&create_empty_value_node("Int", &state).unwrap()), "0");
        assert_eq!(print(&create_empty_value_node("Float!", &state).unwrap()), "0");
        assert_eq!(print(&create_empty_value_node("Boolean", &state).unwrap()), "true");
        assert_eq!(print(&create_empty_value_node("ID", &state).unwrap()), "\"<any id>\"");
        assert_eq!(print(&create_empty_value_node("[ID!]!", &state).unwrap()), "[]");
        assert_eq!(
            create_empty_value_node("Unknown", &state).unwrap_err().message(),
            "Type \"Unknown\" is not defined."
        );
    }

    mod fixture {
        use std::rc::Rc;

        use crate::utils::collections::{IndexMap, IndexSet};

        use crate::specifications::federation::FederationVersion;
        use crate::subgraph::state::{SubgraphFederation, SubgraphGraph, SubgraphSchema, SubgraphSpecs, SubgraphState};
        use crate::supergraph::composition::object_type::{
            FieldStateInGraph, ObjectTypeFieldState, ObjectTypeState, ObjectTypeStateInGraph,
        };
        use crate::supergraph::state::SupergraphState;

        fn field_in_graph(ty: &str) -> FieldStateInGraph {
            FieldStateInGraph {
                r#type: ty.to_string(),
                external: false,
                description: None,
                inaccessible: false,
                used: false,
                r#override: None,
                override_label: None,
                provides: None,
                requires: None,
                provided: false,
                required: false,
                authenticated: false,
                policies: vec![],
                scopes: vec![],
                shareable: false,
                used_as_key: false,
                extension: false,
                version: FederationVersion::V2_3,
            }
        }

        fn field(name: &str, ty: &str, graphs: &[&str]) -> ObjectTypeFieldState {
            ObjectTypeFieldState {
                name: name.to_string(),
                r#type: ty.to_string(),
                tags: IndexSet::default(),
                inaccessible: false,
                is_leaf: false,
                authenticated: false,
                policies: vec![],
                scopes: vec![],
                cost: None,
                list_size: None,
                used_as_key: false,
                r#override: None,
                override_label: None,
                by_graph: graphs.iter().map(|g| (g.to_string(), field_in_graph(ty))).collect(),
                args: IndexMap::default(),
                description: None,
                deprecated: None,
                ast: Default::default(),
                internal_seen_non_external: true,
            }
        }

        fn type_in_graph() -> ObjectTypeStateInGraph {
            ObjectTypeStateInGraph {
                has_definition: true,
                extension: false,
                extension_type: None,
                external: false,
                authenticated: false,
                policies: vec![],
                scopes: vec![],
                keys: vec![],
                interfaces: IndexSet::default(),
                inaccessible: false,
                shareable: false,
                version: FederationVersion::V2_3,
            }
        }

        fn object(name: &str, graphs: &[&str], fields: Vec<ObjectTypeFieldState>) -> ObjectTypeState {
            ObjectTypeState {
                name: name.to_string(),
                tags: IndexSet::default(),
                inaccessible: false,
                authenticated: false,
                policies: vec![],
                scopes: vec![],
                contexts: IndexSet::default(),
                cost: None,
                has_definition: true,
                by_graph: graphs.iter().map(|g| (g.to_string(), type_in_graph())).collect(),
                interfaces: IndexSet::default(),
                fields: fields.into_iter().map(|f| (f.name.clone(), f)).collect(),
                is_entity: false,
                description: None,
                ast: Default::default(),
            }
        }

        fn subgraph(id: &str) -> Rc<SubgraphState> {
            Rc::new(SubgraphState {
                graph: SubgraphGraph {
                    id: id.to_string(),
                    name: id.to_string(),
                    version: FederationVersion::V2_3,
                    url: None,
                },
                types: IndexMap::default(),
                schema: SubgraphSchema::default(),
                links: vec![],
                specs: SubgraphSpecs::default(),
                federation: SubgraphFederation {
                    version: FederationVersion::V2_3,
                    imports: vec![],
                },
            })
        }

        /// Subgraph a: `type Query { user: User } type User { id: ID }`.
        /// Subgraph b: `type User { name: String }` (no `@key`).
        pub fn unreachable_field() -> SupergraphState {
            let mut state = SupergraphState::default();
            state.subgraphs.insert("a".into(), subgraph("a"));
            state.subgraphs.insert("b".into(), subgraph("b"));
            state
                .object_types
                .insert("Query".into(), object("Query", &["a"], vec![field("user", "User", &["a"])]));
            state.object_types.insert(
                "User".into(),
                object(
                    "User",
                    &["a", "b"],
                    vec![field("id", "ID", &["a"]), field("name", "String", &["b"])],
                ),
            );
            state
        }

        /// Both fields of `User` are in subgraph a.
        pub fn satisfiable() -> SupergraphState {
            let mut state = SupergraphState::default();
            state.subgraphs.insert("a".into(), subgraph("a"));
            state
                .object_types
                .insert("Query".into(), object("Query", &["a"], vec![field("user", "User", &["a"])]));
            state.object_types.insert(
                "User".into(),
                object(
                    "User",
                    &["a"],
                    vec![field("id", "ID", &["a"]), field("name", "String", &["a"])],
                ),
            );
            state
        }
    }

    #[test]
    fn satisfiable_supergraph_has_no_unreachable_paths() {
        let state = fixture::satisfiable();
        let supergraph = Supergraph::new(&state).unwrap();
        assert!(supergraph.validate().unwrap().is_empty());

        let trail = supergraph
            .validate_operation(
                graphql_rs::OperationType::Query,
                &[
                    Step::Field {
                        field_name: "user".into(),
                        type_name: "Query".into(),
                    },
                    Step::Field {
                        field_name: "name".into(),
                        type_name: "User".into(),
                    },
                ],
            )
            .unwrap();
        assert!(trail.is_possible());
    }

    #[test]
    fn unreachable_field_reports_missing_field_and_no_key() {
        let state = fixture::unreachable_field();
        let supergraph = Supergraph::new(&state).unwrap();
        let unreachables = supergraph.validate().unwrap();
        assert_eq!(unreachables.len(), 1);

        let graph = supergraph.supergraph_graph();
        let unreachable = &unreachables[0];
        let query_path: QueryPath = unreachable
            .super_path
            .steps(graph)
            .into_iter()
            .map(|step| match step {
                Step::Field { field_name, type_name } => QueryPathEntry {
                    type_name,
                    field_name: Some(field_name),
                },
                Step::Abstract { type_name } => QueryPathEntry {
                    type_name,
                    field_name: None,
                },
            })
            .collect();
        assert_eq!(print_query_path(&state, &query_path).unwrap(), "{\n  user {\n    name\n  }\n}");

        let messages: Vec<String> = unreachable
            .list_errors(graph)
            .unwrap()
            .into_iter()
            .map(|e| format!("{}: {}", e.source_graph_name, e.message))
            .collect();
        assert_eq!(
            messages,
            vec![
                "a: cannot find field \"User.name\".",
                "a: cannot move to subgraph \"b\", which has field \"User.name\", because type \"User\" has no @key defined in subgraph \"b\".",
            ]
        );
    }
}
