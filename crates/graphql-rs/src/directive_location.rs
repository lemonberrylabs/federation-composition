//! graphql-js `DirectiveLocation`.

use std::fmt;

macro_rules! directive_locations {
    ($( $variant:ident = $s:literal ),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum DirectiveLocation {
            $( $variant, )*
        }

        impl DirectiveLocation {
            /// All locations, in graphql-js declaration order.
            pub const ALL: &'static [DirectiveLocation] = &[ $( DirectiveLocation::$variant, )* ];

            pub fn as_str(self) -> &'static str {
                match self {
                    $( DirectiveLocation::$variant => $s, )*
                }
            }

            #[allow(clippy::should_implement_trait)]
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $( $s => Some(DirectiveLocation::$variant), )*
                    _ => None,
                }
            }
        }
    };
}

directive_locations! {
    Query = "QUERY",
    Mutation = "MUTATION",
    Subscription = "SUBSCRIPTION",
    Field = "FIELD",
    FragmentDefinition = "FRAGMENT_DEFINITION",
    FragmentSpread = "FRAGMENT_SPREAD",
    InlineFragment = "INLINE_FRAGMENT",
    VariableDefinition = "VARIABLE_DEFINITION",
    FragmentVariableDefinition = "FRAGMENT_VARIABLE_DEFINITION",
    Schema = "SCHEMA",
    Scalar = "SCALAR",
    Object = "OBJECT",
    FieldDefinition = "FIELD_DEFINITION",
    ArgumentDefinition = "ARGUMENT_DEFINITION",
    Interface = "INTERFACE",
    Union = "UNION",
    Enum = "ENUM",
    EnumValue = "ENUM_VALUE",
    InputObject = "INPUT_OBJECT",
    InputFieldDefinition = "INPUT_FIELD_DEFINITION",
    DirectiveDefinition = "DIRECTIVE_DEFINITION",
}

impl fmt::Display for DirectiveLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
