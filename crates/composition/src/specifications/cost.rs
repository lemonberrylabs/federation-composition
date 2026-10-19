//! Port of `src/specifications/cost.ts`.

/// The `names` argument of `sdl(names)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostNames {
    pub cost: String,
    pub list_size: String,
}

/// `sdl({ cost, listSize })`.
pub fn sdl(names: &CostNames) -> String {
    format!(
        r#"
  directive @{cost}(
    weight: Int!
  ) on ARGUMENT_DEFINITION | ENUM | FIELD_DEFINITION | INPUT_FIELD_DEFINITION | OBJECT | SCALAR

  directive @{list_size}(
    assumedSize: Int
    slicingArguments: [String!]
    sizedFields: [String!]
    requireOneSlicingArgument: Boolean = true
  ) on FIELD_DEFINITION
"#,
        cost = names.cost,
        list_size = names.list_size
    )
}
