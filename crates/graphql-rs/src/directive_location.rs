//! graphql-js `DirectiveLocation`.

use std::fmt;
use std::str::FromStr;

use crate::ast::UnknownName;

macro_rules! directive_locations {
    ($( $variant:ident = $s:literal ),* $(,)?) => {
        /// graphql-js `DirectiveLocation`: where a directive may appear.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum DirectiveLocation {
            $(
                #[doc = concat!("`", $s, "`.")]
                $variant,
            )*
        }

        impl DirectiveLocation {
            /// All locations, in graphql-js declaration order.
            pub const ALL: &'static [DirectiveLocation] = &[ $( DirectiveLocation::$variant, )* ];

            /// The location's name, e.g. `"FIELD_DEFINITION"`.
            pub fn as_str(self) -> &'static str {
                match self {
                    $( DirectiveLocation::$variant => $s, )*
                }
            }
        }

        impl FromStr for DirectiveLocation {
            type Err = UnknownName;

            /// The location named `s`, e.g. `"FIELD_DEFINITION"`.
            fn from_str(s: &str) -> Result<Self, UnknownName> {
                match s {
                    $( $s => Ok(DirectiveLocation::$variant), )*
                    _ => Err(UnknownName::new("directive location", s)),
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
