//! The GraphQL AST, a port of graphql-js 17.0.2 `language/ast`.
//!
//! One struct per graphql-js node kind, named like the graphql-js interface
//! without the `Node` suffix (`FieldDefinitionNode` is [`FieldDefinition`]).
//! Fields are the graphql-js fields in snake_case; `type` is spelled `ty`.
//!
//! Optional list fields (`directives?`, `arguments?`, `fields?`...) are plain
//! `Vec`s: an empty `Vec` stands for both `undefined` and `[]`, which graphql-js
//! code treats the same way. Optional child nodes are `Option`s.
//!
//! Every type implements `Default`, so nodes can be built with struct update
//! syntax: `FieldDefinition { name: Name::new("id"), ty: Type::named("ID").into(), ..Default::default() }`.
//! The `Default` of an enum such as [`Value`] or [`Type`] is only a placeholder.
//!
//! Serde implements the JSON form graphql-js produces with `JSON.stringify`:
//! every node carries its `kind`, keys are camelCase in graphql-js order,
//! optional lists are written only when non-empty and optional nodes only when
//! present. Deserialization accepts optional lists that are missing, `null` or
//! `[]`, accepts keys in any order, and ignores `loc` and unknown keys.

use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, IgnoredAny, MapAccess};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::visitor::{edit_list, edit_one, edit_opt, edit_scalar, EditVal};

/// graphql-js `OperationTypeNode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum OperationType {
    #[default]
    Query,
    Mutation,
    Subscription,
}

impl OperationType {
    pub fn as_str(self) -> &'static str {
        match self {
            OperationType::Query => "query",
            OperationType::Mutation => "mutation",
            OperationType::Subscription => "subscription",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "query" => Some(OperationType::Query),
            "mutation" => Some(OperationType::Mutation),
            "subscription" => Some(OperationType::Subscription),
            _ => None,
        }
    }
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for OperationType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OperationType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = ShortStr::deserialize(d)?;
        OperationType::from_str(s.as_str()).ok_or_else(|| {
            de::Error::invalid_value(
                de::Unexpected::Str(s.as_str()),
                &"\"query\", \"mutation\" or \"subscription\"",
            )
        })
    }
}

/// A string read without allocating, used for JSON keys and `kind` values.
/// Strings longer than the buffer read as empty, which matches no key or kind.
struct ShortStr {
    buf: [u8; 32],
    len: usize,
}

impl ShortStr {
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl<'de> Deserialize<'de> for ShortStr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = ShortStr;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<ShortStr, E> {
                let mut s = ShortStr { buf: [0; 32], len: 0 };
                if v.len() <= s.buf.len() {
                    s.buf[..v.len()].copy_from_slice(v.as_bytes());
                    s.len = v.len();
                }
                Ok(s)
            }
        }
        d.deserialize_str(V)
    }
}

/// Deserialization of a node whose `kind` key may already have been consumed.
pub(crate) trait MapNode: Sized {
    const KIND: Kind;
    fn from_map<'de, A: MapAccess<'de>>(map: A) -> Result<Self, A::Error>;
}

struct StructVisitor<T>(PhantomData<T>);

impl<'de, T: MapNode> de::Visitor<'de> for StructVisitor<T> {
    type Value = T;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a {} node", T::KIND.as_str())
    }
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::from_map(map)
    }
}

fn check_kind<E: de::Error>(found: &ShortStr, expected: Kind) -> Result<(), E> {
    if found.as_str() == expected.as_str() {
        Ok(())
    } else {
        Err(E::invalid_value(
            de::Unexpected::Str(found.as_str()),
            &expected.as_str(),
        ))
    }
}

/// Deserialization of a union of node kinds, dispatching on `kind`.
pub(crate) trait KindDispatch: Sized {
    const EXPECTING: &'static str;
    fn from_map_with_kind<'de, A: MapAccess<'de>>(kind: Kind, map: A) -> Result<Self, A::Error>;
    fn from_value_with_kind(kind: Kind, value: serde_json::Value) -> Result<Self, serde_json::Error>;
}

struct DispatchVisitor<T>(PhantomData<T>);

impl<'de, T: KindDispatch> de::Visitor<'de> for DispatchVisitor<T> {
    type Value = T;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(T::EXPECTING)
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<T, A::Error> {
        let Some(first) = map.next_key::<ShortStr>()? else {
            return Err(de::Error::missing_field("kind"));
        };
        if first.as_str() == "kind" {
            let kind = map.next_value::<ShortStr>()?;
            let kind = Kind::from_str(kind.as_str())
                .ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(kind.as_str()), &T::EXPECTING))?;
            return T::from_map_with_kind(kind, map);
        }
        // `kind` is not the first key: buffer the object to find it.
        let mut object = serde_json::Map::new();
        let first_value = map.next_value::<serde_json::Value>()?;
        object.insert(first.as_str().to_owned(), first_value);
        while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
            object.insert(key, value);
        }
        let kind = match object.get("kind") {
            Some(serde_json::Value::String(kind)) => kind.clone(),
            _ => return Err(de::Error::missing_field("kind")),
        };
        let kind =
            Kind::from_str(&kind).ok_or_else(|| de::Error::invalid_value(de::Unexpected::Str(&kind), &T::EXPECTING))?;
        T::from_value_with_kind(kind, serde_json::Value::Object(object)).map_err(de::Error::custom)
    }
}

/// Borrowed access to a node as a [`NodeRef`].
pub trait AsNodeRef {
    fn as_node_ref(&self) -> NodeRef<'_>;
}

impl<T: AsNodeRef + ?Sized> AsNodeRef for Box<T> {
    fn as_node_ref(&self) -> NodeRef<'_> {
        (**self).as_node_ref()
    }
}

/// Conversion from an owned [`Node`] of a compatible kind.
pub trait FromNode: Sized {
    /// Returns the node back when its kind does not fit `Self`.
    #[allow(clippy::result_large_err)]
    fn from_node(node: Node) -> Result<Self, Node>;
}

impl<T: FromNode> FromNode for Box<T> {
    fn from_node(node: Node) -> Result<Self, Node> {
        T::from_node(node).map(Box::new)
    }
}

/// Element types of AST lists, viewable as a [`ListRef`].
pub trait ListElem: Sized {
    fn list_ref(items: &[Self]) -> ListRef<'_>;
}

/// The child slots of a node, in graphql-js `QueryDocumentKeys` order.
#[derive(Clone, Copy, Debug)]
pub enum SlotRef<'a> {
    /// Not a node field, or an absent optional node.
    Absent,
    One(NodeRef<'a>),
    Many(ListRef<'a>),
}

/// The fields of a node, each with its graphql-js key, in field order.
#[derive(Clone, Copy, Debug)]
pub struct Children<'a> {
    slots: [(&'static str, SlotRef<'a>); 6],
    len: usize,
}

impl<'a> Children<'a> {
    fn new() -> Self {
        Children {
            slots: [("", SlotRef::Absent); 6],
            len: 0,
        }
    }

    fn push(&mut self, key: &'static str, slot: SlotRef<'a>) {
        self.slots[self.len] = (key, slot);
        self.len += 1;
    }

    /// `(key, slot)` pairs; the position of a pair is the field index.
    pub fn as_slice(&self) -> &[(&'static str, SlotRef<'a>)] {
        &self.slots[..self.len]
    }
}

macro_rules! slot {
    (one, $f:expr) => {
        SlotRef::One(AsNodeRef::as_node_ref($f))
    };
    (opt, $f:expr) => {
        match $f {
            Some(x) => SlotRef::One(AsNodeRef::as_node_ref(x)),
            None => SlotRef::Absent,
        }
    };
    (list, $f:expr) => {
        SlotRef::Many(ListElem::list_ref($f.as_slice()))
    };
    (req_list, $f:expr) => {
        SlotRef::Many(ListElem::list_ref($f.as_slice()))
    };
    (scalar, $f:expr) => {
        SlotRef::Absent
    };
    (scalar_default, $f:expr) => {
        SlotRef::Absent
    };
}

macro_rules! ser_field {
    (one, $m:ident, $key:literal, $f:expr) => {
        $m.serialize_entry($key, $f)?
    };
    (opt, $m:ident, $key:literal, $f:expr) => {
        if let Some(x) = $f {
            $m.serialize_entry($key, x)?
        }
    };
    (list, $m:ident, $key:literal, $f:expr) => {
        if !$f.is_empty() {
            $m.serialize_entry($key, $f)?
        }
    };
    (req_list, $m:ident, $key:literal, $f:expr) => {
        $m.serialize_entry($key, $f)?
    };
    (scalar, $m:ident, $key:literal, $f:expr) => {
        $m.serialize_entry($key, $f)?
    };
    (scalar_default, $m:ident, $key:literal, $f:expr) => {
        $m.serialize_entry($key, $f)?
    };
}

macro_rules! de_field {
    (one, $map:ident, $ty:ty) => {
        $map.next_value::<$ty>()?
    };
    (opt, $map:ident, $ty:ty) => {
        $map.next_value::<$ty>()?
    };
    (list, $map:ident, $ty:ty) => {
        $map.next_value::<Option<$ty>>()?.unwrap_or_default()
    };
    (req_list, $map:ident, $ty:ty) => {
        $map.next_value::<Option<$ty>>()?.unwrap_or_default()
    };
    (scalar, $map:ident, $ty:ty) => {
        $map.next_value::<$ty>()?
    };
    (scalar_default, $map:ident, $ty:ty) => {
        $map.next_value::<Option<$ty>>()?.unwrap_or_default()
    };
}

macro_rules! de_finish {
    (one, $v:ident, $key:literal) => {
        match $v {
            Some(x) => x,
            None => return Err(de::Error::missing_field($key)),
        }
    };
    (opt, $v:ident, $key:literal) => {
        $v.unwrap_or_default()
    };
    (list, $v:ident, $key:literal) => {
        $v.unwrap_or_default()
    };
    (req_list, $v:ident, $key:literal) => {
        $v.unwrap_or_default()
    };
    (scalar, $v:ident, $key:literal) => {
        match $v {
            Some(x) => x,
            None => return Err(de::Error::missing_field($key)),
        }
    };
    (scalar_default, $v:ident, $key:literal) => {
        $v.unwrap_or_default()
    };
}

macro_rules! edit_conv {
    (one, $v:expr, $kind:literal, $key:literal) => {
        edit_one($v, $kind, $key)
    };
    (opt, $v:expr, $kind:literal, $key:literal) => {
        edit_opt($v, $kind, $key)
    };
    (list, $v:expr, $kind:literal, $key:literal) => {
        edit_list($v, $kind, $key)
    };
    (req_list, $v:expr, $kind:literal, $key:literal) => {
        edit_list($v, $kind, $key)
    };
    (scalar, $v:expr, $kind:literal, $key:literal) => {
        edit_scalar($v)
    };
    (scalar_default, $v:expr, $kind:literal, $key:literal) => {
        edit_scalar($v)
    };
}

macro_rules! define_nodes {
    ($(
        $(#[$meta:meta])*
        $name:ident $kind:literal {
            $( $(#[$fmeta:meta])* $field:ident : $fty:ty = $cat:ident $key:literal ),* $(,)?
        }
    )*) => {
        $(
            $(#[$meta])*
            #[derive(Clone, Debug, PartialEq, Default)]
            pub struct $name {
                $( $(#[$fmeta])* pub $field: $fty, )*
            }

            impl $name {
                pub const KIND: Kind = Kind::$name;

                /// The node's fields with their graphql-js keys, in visiting order.
                pub fn children(&self) -> Children<'_> {
                    #[allow(unused_mut)]
                    let mut c = Children::new();
                    $( c.push($key, slot!($cat, &self.$field)); )*
                    c
                }

                /// Builds a copy with the given field edits applied (last edit
                /// per field wins); untouched fields are cloned.
                #[allow(unused_variables, unused_mut, unused_assignments, clippy::all)]
                pub(crate) fn rebuild(&self, edits: Vec<(usize, EditVal)>) -> Self {
                    $( let mut $field: Option<$fty> = None; )*
                    for (i, v) in edits {
                        let mut v = Some(v);
                        let mut idx = 0usize;
                        $(
                            if i == idx {
                                $field = Some(edit_conv!($cat, v.take().unwrap(), $kind, $key));
                            }
                            idx += 1;
                        )*
                    }
                    $name {
                        $( $field: match $field { Some(x) => x, None => self.$field.clone() }, )*
                    }
                }
            }

            impl Serialize for $name {
                fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                    let mut m = s.serialize_map(None)?;
                    m.serialize_entry("kind", $kind)?;
                    $( ser_field!($cat, m, $key, &self.$field); )*
                    m.end()
                }
            }

            impl MapNode for $name {
                const KIND: Kind = Kind::$name;
                #[allow(unused_mut)]
                fn from_map<'de, A: MapAccess<'de>>(mut map: A) -> Result<Self, A::Error> {
                    $( let mut $field: Option<$fty> = None; )*
                    while let Some(key) = map.next_key::<ShortStr>()? {
                        match key.as_str() {
                            "kind" => {
                                let kind = map.next_value::<ShortStr>()?;
                                check_kind::<A::Error>(&kind, Kind::$name)?;
                            }
                            $( $key => { $field = Some(de_field!($cat, map, $fty)); } )*
                            _ => {
                                map.next_value::<IgnoredAny>()?;
                            }
                        }
                    }
                    Ok($name { $( $field: de_finish!($cat, $field, $key), )* })
                }
            }

            impl<'de> Deserialize<'de> for $name {
                fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                    d.deserialize_map(StructVisitor::<$name>(PhantomData))
                }
            }

            impl AsNodeRef for $name {
                fn as_node_ref(&self) -> NodeRef<'_> {
                    NodeRef::$name(self)
                }
            }

            impl FromNode for $name {
                fn from_node(node: Node) -> Result<Self, Node> {
                    match node {
                        Node::$name(x) => Ok(x),
                        other => Err(other),
                    }
                }
            }

            impl From<$name> for Node {
                fn from(x: $name) -> Node {
                    Node::$name(x)
                }
            }

            impl<'a> From<&'a $name> for NodeRef<'a> {
                fn from(x: &'a $name) -> NodeRef<'a> {
                    NodeRef::$name(x)
                }
            }
        )*

        /// graphql-js `Kind`: the `kind` of every AST node.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Kind {
            $( $name, )*
        }

        impl Kind {
            /// The graphql-js `Kind` string, e.g. `"FieldDefinition"`.
            pub fn as_str(self) -> &'static str {
                match self {
                    $( Kind::$name => $kind, )*
                }
            }

            #[allow(clippy::should_implement_trait)]
            pub fn from_str(s: &str) -> Option<Kind> {
                match s {
                    $( $kind => Some(Kind::$name), )*
                    _ => None,
                }
            }

            /// All kinds, in graphql-js `Kind` order.
            pub const ALL: &'static [Kind] = &[ $( Kind::$name, )* ];
        }

        /// graphql-js `ASTNode`, owned.
        #[derive(Clone, Debug, PartialEq)]
        pub enum Node {
            $( $name($name), )*
        }

        /// graphql-js `ASTNode`, borrowed.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum NodeRef<'a> {
            $( $name(&'a $name), )*
        }

        impl Node {
            pub fn kind(&self) -> Kind {
                match self {
                    $( Node::$name(_) => Kind::$name, )*
                }
            }

            pub fn as_ref(&self) -> NodeRef<'_> {
                match self {
                    $( Node::$name(x) => NodeRef::$name(x), )*
                }
            }
        }

        impl<'a> NodeRef<'a> {
            pub fn kind(self) -> Kind {
                match self {
                    $( NodeRef::$name(_) => Kind::$name, )*
                }
            }

            /// The node's fields with their graphql-js keys, in visiting order.
            pub fn children(self) -> Children<'a> {
                match self {
                    $( NodeRef::$name(x) => x.children(), )*
                }
            }

            pub fn to_owned_node(self) -> Node {
                match self {
                    $( NodeRef::$name(x) => Node::$name(x.clone()), )*
                }
            }

            pub(crate) fn rebuild(self, edits: Vec<(usize, EditVal)>) -> Node {
                match self {
                    $( NodeRef::$name(x) => Node::$name(x.rebuild(edits)), )*
                }
            }
        }

        impl Serialize for Node {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                match self {
                    $( Node::$name(x) => x.serialize(s), )*
                }
            }
        }

        impl KindDispatch for Node {
            const EXPECTING: &'static str = "an AST node";
            fn from_map_with_kind<'de, A: MapAccess<'de>>(kind: Kind, map: A) -> Result<Self, A::Error> {
                match kind {
                    $( Kind::$name => <$name as MapNode>::from_map(map).map(Node::$name), )*
                }
            }
            fn from_value_with_kind(kind: Kind, value: serde_json::Value) -> Result<Self, serde_json::Error> {
                match kind {
                    $( Kind::$name => $name::deserialize(value).map(Node::$name), )*
                }
            }
        }
    };
}

impl AsNodeRef for Node {
    fn as_node_ref(&self) -> NodeRef<'_> {
        self.as_ref()
    }
}

impl FromNode for Node {
    fn from_node(node: Node) -> Result<Self, Node> {
        Ok(node)
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(DispatchVisitor::<Node>(PhantomData))
    }
}

impl<'a> From<&'a Node> for NodeRef<'a> {
    fn from(x: &'a Node) -> NodeRef<'a> {
        x.as_ref()
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> NodeRef<'a> {
    /// The `Name` of nodes that have a `name: NameNode` field.
    pub fn name(self) -> Option<&'a Name> {
        Some(match self {
            NodeRef::OperationDefinition(n) => return n.name.as_ref(),
            NodeRef::Variable(n) => &n.name,
            NodeRef::Field(n) => &n.name,
            NodeRef::Argument(n) => &n.name,
            NodeRef::FragmentArgument(n) => &n.name,
            NodeRef::FragmentSpread(n) => &n.name,
            NodeRef::FragmentDefinition(n) => &n.name,
            NodeRef::ObjectField(n) => &n.name,
            NodeRef::Directive(n) => &n.name,
            NodeRef::NamedType(n) => &n.name,
            NodeRef::ScalarTypeDefinition(n) => &n.name,
            NodeRef::ObjectTypeDefinition(n) => &n.name,
            NodeRef::FieldDefinition(n) => &n.name,
            NodeRef::InputValueDefinition(n) => &n.name,
            NodeRef::InterfaceTypeDefinition(n) => &n.name,
            NodeRef::UnionTypeDefinition(n) => &n.name,
            NodeRef::EnumTypeDefinition(n) => &n.name,
            NodeRef::EnumValueDefinition(n) => &n.name,
            NodeRef::InputObjectTypeDefinition(n) => &n.name,
            NodeRef::DirectiveDefinition(n) => &n.name,
            NodeRef::DirectiveExtension(n) => &n.name,
            NodeRef::ScalarTypeExtension(n) => &n.name,
            NodeRef::ObjectTypeExtension(n) => &n.name,
            NodeRef::InterfaceTypeExtension(n) => &n.name,
            NodeRef::UnionTypeExtension(n) => &n.name,
            NodeRef::EnumTypeExtension(n) => &n.name,
            NodeRef::InputObjectTypeExtension(n) => &n.name,
            NodeRef::TypeCoordinate(n) => &n.name,
            NodeRef::MemberCoordinate(n) => &n.name,
            NodeRef::ArgumentCoordinate(n) => &n.name,
            NodeRef::DirectiveCoordinate(n) => &n.name,
            NodeRef::DirectiveArgumentCoordinate(n) => &n.name,
            _ => return None,
        })
    }

    /// Identity of the referenced node object (kind and address).
    pub fn ptr_eq(self, other: NodeRef<'_>) -> bool {
        self.kind() == other.kind() && self.addr() == other.addr()
    }

    fn addr(self) -> *const () {
        macro_rules! addr {
            ($($v:ident)*) => {
                match self { $( NodeRef::$v(x) => x as *const $v as *const (), )* }
            };
        }
        addr!(Name Document OperationDefinition VariableDefinition SelectionSet Field Argument
            FragmentArgument FragmentSpread InlineFragment FragmentDefinition Variable IntValue
            FloatValue StringValue BooleanValue NullValue EnumValue ListValue ObjectValue
            ObjectField Directive NamedType ListType NonNullType SchemaDefinition
            OperationTypeDefinition ScalarTypeDefinition ObjectTypeDefinition FieldDefinition
            InputValueDefinition InterfaceTypeDefinition UnionTypeDefinition EnumTypeDefinition
            EnumValueDefinition InputObjectTypeDefinition DirectiveDefinition SchemaExtension
            DirectiveExtension ScalarTypeExtension ObjectTypeExtension InterfaceTypeExtension
            UnionTypeExtension EnumTypeExtension InputObjectTypeExtension TypeCoordinate
            MemberCoordinate ArgumentCoordinate DirectiveCoordinate DirectiveArgumentCoordinate)
    }
}

define_nodes! {
    /// graphql-js `NameNode`.
    Name "Name" {
        value: String = scalar "value",
    }

    /// graphql-js `DocumentNode`.
    Document "Document" {
        definitions: Vec<Definition> = req_list "definitions",
    }

    /// graphql-js `OperationDefinitionNode`.
    OperationDefinition "OperationDefinition" {
        operation: OperationType = scalar "operation",
        description: Option<StringValue> = opt "description",
        name: Option<Name> = opt "name",
        variable_definitions: Vec<VariableDefinition> = list "variableDefinitions",
        directives: Vec<Directive> = list "directives",
        selection_set: SelectionSet = one "selectionSet",
    }

    /// graphql-js `VariableDefinitionNode`.
    VariableDefinition "VariableDefinition" {
        description: Option<StringValue> = opt "description",
        variable: Variable = one "variable",
        ty: Type = one "type",
        default_value: Option<Value> = opt "defaultValue",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `SelectionSetNode`.
    SelectionSet "SelectionSet" {
        selections: Vec<Selection> = req_list "selections",
    }

    /// graphql-js `FieldNode`.
    Field "Field" {
        alias: Option<Name> = opt "alias",
        name: Name = one "name",
        arguments: Vec<Argument> = list "arguments",
        directives: Vec<Directive> = list "directives",
        selection_set: Option<SelectionSet> = opt "selectionSet",
    }

    /// graphql-js `ArgumentNode` (and `ConstArgumentNode`).
    Argument "Argument" {
        name: Name = one "name",
        value: Value = one "value",
    }

    /// graphql-js `FragmentArgumentNode` (experimental fragment arguments).
    FragmentArgument "FragmentArgument" {
        name: Name = one "name",
        value: Value = one "value",
    }

    /// graphql-js `FragmentSpreadNode`.
    FragmentSpread "FragmentSpread" {
        name: Name = one "name",
        arguments: Vec<FragmentArgument> = list "arguments",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `InlineFragmentNode`.
    InlineFragment "InlineFragment" {
        type_condition: Option<NamedType> = opt "typeCondition",
        directives: Vec<Directive> = list "directives",
        selection_set: SelectionSet = one "selectionSet",
    }

    /// graphql-js `FragmentDefinitionNode`.
    FragmentDefinition "FragmentDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        variable_definitions: Vec<VariableDefinition> = list "variableDefinitions",
        type_condition: NamedType = one "typeCondition",
        directives: Vec<Directive> = list "directives",
        selection_set: SelectionSet = one "selectionSet",
    }

    /// graphql-js `VariableNode`.
    Variable "Variable" {
        name: Name = one "name",
    }

    /// graphql-js `IntValueNode`; `value` is the source text.
    IntValue "IntValue" {
        value: String = scalar "value",
    }

    /// graphql-js `FloatValueNode`; `value` is the source text.
    FloatValue "FloatValue" {
        value: String = scalar "value",
    }

    /// graphql-js `StringValueNode`.
    StringValue "StringValue" {
        value: String = scalar "value",
        block: bool = scalar_default "block",
    }

    /// graphql-js `BooleanValueNode`.
    BooleanValue "BooleanValue" {
        value: bool = scalar "value",
    }

    /// graphql-js `NullValueNode`.
    NullValue "NullValue" {}

    /// graphql-js `EnumValueNode`.
    EnumValue "EnumValue" {
        value: String = scalar "value",
    }

    /// graphql-js `ListValueNode`.
    ListValue "ListValue" {
        values: Vec<Value> = req_list "values",
    }

    /// graphql-js `ObjectValueNode`.
    ObjectValue "ObjectValue" {
        fields: Vec<ObjectField> = req_list "fields",
    }

    /// graphql-js `ObjectFieldNode`.
    ObjectField "ObjectField" {
        name: Name = one "name",
        value: Value = one "value",
    }

    /// graphql-js `DirectiveNode` (and `ConstDirectiveNode`).
    Directive "Directive" {
        name: Name = one "name",
        arguments: Vec<Argument> = list "arguments",
    }

    /// graphql-js `NamedTypeNode`.
    NamedType "NamedType" {
        name: Name = one "name",
    }

    /// graphql-js `ListTypeNode`.
    ListType "ListType" {
        ty: Box<Type> = one "type",
    }

    /// graphql-js `NonNullTypeNode`. `ty` is never itself a `NonNull`.
    NonNullType "NonNullType" {
        ty: Box<Type> = one "type",
    }

    /// graphql-js `SchemaDefinitionNode`.
    SchemaDefinition "SchemaDefinition" {
        description: Option<StringValue> = opt "description",
        directives: Vec<Directive> = list "directives",
        operation_types: Vec<OperationTypeDefinition> = req_list "operationTypes",
    }

    /// graphql-js `OperationTypeDefinitionNode`.
    OperationTypeDefinition "OperationTypeDefinition" {
        operation: OperationType = scalar "operation",
        ty: NamedType = one "type",
    }

    /// graphql-js `ScalarTypeDefinitionNode`.
    ScalarTypeDefinition "ScalarTypeDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `ObjectTypeDefinitionNode`.
    ObjectTypeDefinition "ObjectTypeDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        interfaces: Vec<NamedType> = list "interfaces",
        directives: Vec<Directive> = list "directives",
        fields: Vec<FieldDefinition> = list "fields",
    }

    /// graphql-js `FieldDefinitionNode`.
    FieldDefinition "FieldDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        arguments: Vec<InputValueDefinition> = list "arguments",
        ty: Type = one "type",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `InputValueDefinitionNode`.
    InputValueDefinition "InputValueDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        ty: Type = one "type",
        default_value: Option<Value> = opt "defaultValue",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `InterfaceTypeDefinitionNode`.
    InterfaceTypeDefinition "InterfaceTypeDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        interfaces: Vec<NamedType> = list "interfaces",
        directives: Vec<Directive> = list "directives",
        fields: Vec<FieldDefinition> = list "fields",
    }

    /// graphql-js `UnionTypeDefinitionNode`.
    UnionTypeDefinition "UnionTypeDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
        types: Vec<NamedType> = list "types",
    }

    /// graphql-js `EnumTypeDefinitionNode`.
    EnumTypeDefinition "EnumTypeDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
        values: Vec<EnumValueDefinition> = list "values",
    }

    /// graphql-js `EnumValueDefinitionNode`.
    EnumValueDefinition "EnumValueDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `InputObjectTypeDefinitionNode`.
    InputObjectTypeDefinition "InputObjectTypeDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
        fields: Vec<InputValueDefinition> = list "fields",
    }

    /// graphql-js `DirectiveDefinitionNode`.
    DirectiveDefinition "DirectiveDefinition" {
        description: Option<StringValue> = opt "description",
        name: Name = one "name",
        arguments: Vec<InputValueDefinition> = list "arguments",
        directives: Vec<Directive> = list "directives",
        repeatable: bool = scalar_default "repeatable",
        locations: Vec<Name> = req_list "locations",
    }

    /// graphql-js `SchemaExtensionNode`.
    SchemaExtension "SchemaExtension" {
        directives: Vec<Directive> = list "directives",
        operation_types: Vec<OperationTypeDefinition> = list "operationTypes",
    }

    /// graphql-js `DirectiveExtensionNode`.
    DirectiveExtension "DirectiveExtension" {
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `ScalarTypeExtensionNode`.
    ScalarTypeExtension "ScalarTypeExtension" {
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
    }

    /// graphql-js `ObjectTypeExtensionNode`.
    ObjectTypeExtension "ObjectTypeExtension" {
        name: Name = one "name",
        interfaces: Vec<NamedType> = list "interfaces",
        directives: Vec<Directive> = list "directives",
        fields: Vec<FieldDefinition> = list "fields",
    }

    /// graphql-js `InterfaceTypeExtensionNode`.
    InterfaceTypeExtension "InterfaceTypeExtension" {
        name: Name = one "name",
        interfaces: Vec<NamedType> = list "interfaces",
        directives: Vec<Directive> = list "directives",
        fields: Vec<FieldDefinition> = list "fields",
    }

    /// graphql-js `UnionTypeExtensionNode`.
    UnionTypeExtension "UnionTypeExtension" {
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
        types: Vec<NamedType> = list "types",
    }

    /// graphql-js `EnumTypeExtensionNode`.
    EnumTypeExtension "EnumTypeExtension" {
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
        values: Vec<EnumValueDefinition> = list "values",
    }

    /// graphql-js `InputObjectTypeExtensionNode`.
    InputObjectTypeExtension "InputObjectTypeExtension" {
        name: Name = one "name",
        directives: Vec<Directive> = list "directives",
        fields: Vec<InputValueDefinition> = list "fields",
    }

    /// graphql-js `TypeCoordinateNode`.
    TypeCoordinate "TypeCoordinate" {
        name: Name = one "name",
    }

    /// graphql-js `MemberCoordinateNode`.
    MemberCoordinate "MemberCoordinate" {
        name: Name = one "name",
        member_name: Name = one "memberName",
    }

    /// graphql-js `ArgumentCoordinateNode`.
    ArgumentCoordinate "ArgumentCoordinate" {
        name: Name = one "name",
        field_name: Name = one "fieldName",
        argument_name: Name = one "argumentName",
    }

    /// graphql-js `DirectiveCoordinateNode`.
    DirectiveCoordinate "DirectiveCoordinate" {
        name: Name = one "name",
    }

    /// graphql-js `DirectiveArgumentCoordinateNode`.
    DirectiveArgumentCoordinate "DirectiveArgumentCoordinate" {
        name: Name = one "name",
        argument_name: Name = one "argumentName",
    }
}

macro_rules! node_enum {
    (
        $(#[$meta:meta])*
        $enum:ident, $expecting:literal { $( $variant:ident($ty:ident) ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        pub enum $enum {
            $( $variant($ty), )*
        }

        impl $enum {
            pub fn kind(&self) -> Kind {
                match self {
                    $( $enum::$variant(_) => Kind::$ty, )*
                }
            }

            pub fn as_node_ref(&self) -> NodeRef<'_> {
                match self {
                    $( $enum::$variant(x) => NodeRef::$ty(x), )*
                }
            }
        }

        impl AsNodeRef for $enum {
            fn as_node_ref(&self) -> NodeRef<'_> {
                $enum::as_node_ref(self)
            }
        }

        impl FromNode for $enum {
            fn from_node(node: Node) -> Result<Self, Node> {
                match node {
                    $( Node::$ty(x) => Ok($enum::$variant(x)), )*
                    other => Err(other),
                }
            }
        }

        $(
            impl From<$ty> for $enum {
                fn from(x: $ty) -> Self {
                    $enum::$variant(x)
                }
            }
        )*

        impl From<$enum> for Node {
            fn from(x: $enum) -> Node {
                match x {
                    $( $enum::$variant(x) => Node::$ty(x), )*
                }
            }
        }

        impl<'a> From<&'a $enum> for NodeRef<'a> {
            fn from(x: &'a $enum) -> NodeRef<'a> {
                x.as_node_ref()
            }
        }

        impl Serialize for $enum {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                match self {
                    $( $enum::$variant(x) => x.serialize(s), )*
                }
            }
        }

        impl KindDispatch for $enum {
            const EXPECTING: &'static str = $expecting;
            fn from_map_with_kind<'de, A: MapAccess<'de>>(kind: Kind, map: A) -> Result<Self, A::Error> {
                match kind {
                    $( Kind::$ty => <$ty as MapNode>::from_map(map).map($enum::$variant), )*
                    other => Err(de::Error::invalid_value(de::Unexpected::Str(other.as_str()), &$expecting)),
                }
            }
            fn from_value_with_kind(kind: Kind, value: serde_json::Value) -> Result<Self, serde_json::Error> {
                match kind {
                    $( Kind::$ty => $ty::deserialize(value).map($enum::$variant), )*
                    other => Err(de::Error::invalid_value(de::Unexpected::Str(other.as_str()), &$expecting)),
                }
            }
        }

        impl<'de> Deserialize<'de> for $enum {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                d.deserialize_map(DispatchVisitor::<$enum>(PhantomData))
            }
        }
    };
}

node_enum! {
    /// graphql-js `DefinitionNode`.
    Definition, "a definition node" {
        OperationDefinition(OperationDefinition),
        FragmentDefinition(FragmentDefinition),
        SchemaDefinition(SchemaDefinition),
        ScalarTypeDefinition(ScalarTypeDefinition),
        ObjectTypeDefinition(ObjectTypeDefinition),
        InterfaceTypeDefinition(InterfaceTypeDefinition),
        UnionTypeDefinition(UnionTypeDefinition),
        EnumTypeDefinition(EnumTypeDefinition),
        InputObjectTypeDefinition(InputObjectTypeDefinition),
        DirectiveDefinition(DirectiveDefinition),
        SchemaExtension(SchemaExtension),
        DirectiveExtension(DirectiveExtension),
        ScalarTypeExtension(ScalarTypeExtension),
        ObjectTypeExtension(ObjectTypeExtension),
        InterfaceTypeExtension(InterfaceTypeExtension),
        UnionTypeExtension(UnionTypeExtension),
        EnumTypeExtension(EnumTypeExtension),
        InputObjectTypeExtension(InputObjectTypeExtension),
    }
}

node_enum! {
    /// graphql-js `TypeDefinitionNode`.
    TypeDefinition, "a type definition node" {
        ScalarTypeDefinition(ScalarTypeDefinition),
        ObjectTypeDefinition(ObjectTypeDefinition),
        InterfaceTypeDefinition(InterfaceTypeDefinition),
        UnionTypeDefinition(UnionTypeDefinition),
        EnumTypeDefinition(EnumTypeDefinition),
        InputObjectTypeDefinition(InputObjectTypeDefinition),
    }
}

node_enum! {
    /// graphql-js `TypeExtensionNode`.
    TypeExtension, "a type extension node" {
        ScalarTypeExtension(ScalarTypeExtension),
        ObjectTypeExtension(ObjectTypeExtension),
        InterfaceTypeExtension(InterfaceTypeExtension),
        UnionTypeExtension(UnionTypeExtension),
        EnumTypeExtension(EnumTypeExtension),
        InputObjectTypeExtension(InputObjectTypeExtension),
    }
}

node_enum! {
    /// graphql-js `SelectionNode`.
    Selection, "a selection node" {
        Field(Field),
        FragmentSpread(FragmentSpread),
        InlineFragment(InlineFragment),
    }
}

node_enum! {
    /// graphql-js `ValueNode` (and `ConstValueNode`).
    Value, "a value node" {
        Variable(Variable),
        Int(IntValue),
        Float(FloatValue),
        String(StringValue),
        Boolean(BooleanValue),
        Null(NullValue),
        Enum(EnumValue),
        List(ListValue),
        Object(ObjectValue),
    }
}

node_enum! {
    /// graphql-js `TypeNode`.
    Type, "a type node" {
        Named(NamedType),
        List(ListType),
        NonNull(NonNullType),
    }
}

node_enum! {
    /// graphql-js `SchemaCoordinateNode`.
    SchemaCoordinate, "a schema coordinate node" {
        TypeCoordinate(TypeCoordinate),
        MemberCoordinate(MemberCoordinate),
        ArgumentCoordinate(ArgumentCoordinate),
        DirectiveCoordinate(DirectiveCoordinate),
        DirectiveArgumentCoordinate(DirectiveArgumentCoordinate),
    }
}

/// graphql-js `ConstValueNode`; the same type as [`Value`].
pub type ConstValue = Value;
/// graphql-js `ConstDirectiveNode`; the same type as [`Directive`].
pub type ConstDirective = Directive;
/// graphql-js `ConstArgumentNode`; the same type as [`Argument`].
pub type ConstArgument = Argument;
/// graphql-js `ConstObjectFieldNode`; the same type as [`ObjectField`].
pub type ConstObjectField = ObjectField;

impl Default for Value {
    fn default() -> Self {
        Value::Null(NullValue {})
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::Named(NamedType::default())
    }
}

impl From<TypeDefinition> for Definition {
    fn from(x: TypeDefinition) -> Self {
        match x {
            TypeDefinition::ScalarTypeDefinition(x) => Definition::ScalarTypeDefinition(x),
            TypeDefinition::ObjectTypeDefinition(x) => Definition::ObjectTypeDefinition(x),
            TypeDefinition::InterfaceTypeDefinition(x) => Definition::InterfaceTypeDefinition(x),
            TypeDefinition::UnionTypeDefinition(x) => Definition::UnionTypeDefinition(x),
            TypeDefinition::EnumTypeDefinition(x) => Definition::EnumTypeDefinition(x),
            TypeDefinition::InputObjectTypeDefinition(x) => Definition::InputObjectTypeDefinition(x),
        }
    }
}

impl From<TypeExtension> for Definition {
    fn from(x: TypeExtension) -> Self {
        match x {
            TypeExtension::ScalarTypeExtension(x) => Definition::ScalarTypeExtension(x),
            TypeExtension::ObjectTypeExtension(x) => Definition::ObjectTypeExtension(x),
            TypeExtension::InterfaceTypeExtension(x) => Definition::InterfaceTypeExtension(x),
            TypeExtension::UnionTypeExtension(x) => Definition::UnionTypeExtension(x),
            TypeExtension::EnumTypeExtension(x) => Definition::EnumTypeExtension(x),
            TypeExtension::InputObjectTypeExtension(x) => Definition::InputObjectTypeExtension(x),
        }
    }
}

impl TryFrom<Definition> for TypeDefinition {
    type Error = Definition;
    fn try_from(x: Definition) -> Result<Self, Definition> {
        Ok(match x {
            Definition::ScalarTypeDefinition(x) => TypeDefinition::ScalarTypeDefinition(x),
            Definition::ObjectTypeDefinition(x) => TypeDefinition::ObjectTypeDefinition(x),
            Definition::InterfaceTypeDefinition(x) => TypeDefinition::InterfaceTypeDefinition(x),
            Definition::UnionTypeDefinition(x) => TypeDefinition::UnionTypeDefinition(x),
            Definition::EnumTypeDefinition(x) => TypeDefinition::EnumTypeDefinition(x),
            Definition::InputObjectTypeDefinition(x) => TypeDefinition::InputObjectTypeDefinition(x),
            other => return Err(other),
        })
    }
}

impl TryFrom<Definition> for TypeExtension {
    type Error = Definition;
    fn try_from(x: Definition) -> Result<Self, Definition> {
        Ok(match x {
            Definition::ScalarTypeExtension(x) => TypeExtension::ScalarTypeExtension(x),
            Definition::ObjectTypeExtension(x) => TypeExtension::ObjectTypeExtension(x),
            Definition::InterfaceTypeExtension(x) => TypeExtension::InterfaceTypeExtension(x),
            Definition::UnionTypeExtension(x) => TypeExtension::UnionTypeExtension(x),
            Definition::EnumTypeExtension(x) => TypeExtension::EnumTypeExtension(x),
            Definition::InputObjectTypeExtension(x) => TypeExtension::InputObjectTypeExtension(x),
            other => return Err(other),
        })
    }
}

impl TypeDefinition {
    pub fn name(&self) -> &Name {
        match self {
            TypeDefinition::ScalarTypeDefinition(x) => &x.name,
            TypeDefinition::ObjectTypeDefinition(x) => &x.name,
            TypeDefinition::InterfaceTypeDefinition(x) => &x.name,
            TypeDefinition::UnionTypeDefinition(x) => &x.name,
            TypeDefinition::EnumTypeDefinition(x) => &x.name,
            TypeDefinition::InputObjectTypeDefinition(x) => &x.name,
        }
    }
}

impl TypeExtension {
    pub fn name(&self) -> &Name {
        match self {
            TypeExtension::ScalarTypeExtension(x) => &x.name,
            TypeExtension::ObjectTypeExtension(x) => &x.name,
            TypeExtension::InterfaceTypeExtension(x) => &x.name,
            TypeExtension::UnionTypeExtension(x) => &x.name,
            TypeExtension::EnumTypeExtension(x) => &x.name,
            TypeExtension::InputObjectTypeExtension(x) => &x.name,
        }
    }
}

impl Definition {
    /// The `name` of every definition kind that has one (all but schema
    /// definitions/extensions and anonymous operations).
    pub fn name(&self) -> Option<&Name> {
        self.as_node_ref().name()
    }
}

macro_rules! list_elems {
    ($( $ty:ident => $variant:ident ),* $(,)?) => {
        /// A borrowed AST list (graphql-js `ReadonlyArray<ASTNode>`).
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum ListRef<'a> {
            $( $variant(&'a [$ty]), )*
        }

        impl<'a> ListRef<'a> {
            pub fn len(self) -> usize {
                match self {
                    $( ListRef::$variant(s) => s.len(), )*
                }
            }

            pub fn is_empty(self) -> bool {
                self.len() == 0
            }

            pub fn get(self, index: usize) -> Option<NodeRef<'a>> {
                match self {
                    $( ListRef::$variant(s) => s.get(index).map(|x| x.as_node_ref()), )*
                }
            }

            pub fn iter(self) -> impl Iterator<Item = NodeRef<'a>> {
                (0..self.len()).map(move |i| self.get(i).unwrap())
            }

            /// Identity of the referenced slice (element type, address, length).
            pub fn ptr_eq(self, other: ListRef<'_>) -> bool {
                fn raw<'b>(l: ListRef<'b>) -> (usize, *const (), usize) {
                    let mut tag = 0usize;
                    $(
                        if let ListRef::$variant(s) = l {
                            return (tag, s.as_ptr() as *const (), s.len());
                        }
                        tag += 1;
                    )*
                    let _ = tag;
                    unreachable!()
                }
                raw(self) == raw(other)
            }
        }

        $(
            impl ListElem for $ty {
                fn list_ref(items: &[Self]) -> ListRef<'_> {
                    ListRef::$variant(items)
                }
            }
        )*
    };
}

list_elems! {
    Definition => Definitions,
    Selection => Selections,
    VariableDefinition => VariableDefinitions,
    Argument => Arguments,
    FragmentArgument => FragmentArguments,
    Directive => Directives,
    Value => Values,
    ObjectField => ObjectFields,
    NamedType => NamedTypes,
    Name => Names,
    OperationTypeDefinition => OperationTypeDefinitions,
    FieldDefinition => FieldDefinitions,
    InputValueDefinition => InputValueDefinitions,
    EnumValueDefinition => EnumValueDefinitions,
}

// Concise constructors.

impl Name {
    pub fn new(value: impl Into<String>) -> Self {
        Name { value: value.into() }
    }
}

impl NamedType {
    pub fn new(name: impl Into<String>) -> Self {
        NamedType { name: Name::new(name) }
    }
}

impl StringValue {
    pub fn new(value: impl Into<String>, block: bool) -> Self {
        StringValue {
            value: value.into(),
            block,
        }
    }
}

impl Variable {
    pub fn new(name: impl Into<String>) -> Self {
        Variable { name: Name::new(name) }
    }
}

impl Argument {
    pub fn new(name: impl Into<String>, value: Value) -> Self {
        Argument {
            name: Name::new(name),
            value,
        }
    }
}

impl ObjectField {
    pub fn new(name: impl Into<String>, value: Value) -> Self {
        ObjectField {
            name: Name::new(name),
            value,
        }
    }
}

impl Directive {
    pub fn new(name: impl Into<String>, arguments: Vec<Argument>) -> Self {
        Directive {
            name: Name::new(name),
            arguments,
        }
    }
}

impl Document {
    pub fn new(definitions: Vec<Definition>) -> Self {
        Document { definitions }
    }
}

impl Type {
    pub fn named(name: impl Into<String>) -> Self {
        Type::Named(NamedType::new(name))
    }

    pub fn list(of: Type) -> Self {
        Type::List(ListType { ty: Box::new(of) })
    }

    pub fn non_null(of: Type) -> Self {
        Type::NonNull(NonNullType { ty: Box::new(of) })
    }
}

impl Value {
    pub fn string(value: impl Into<String>) -> Self {
        Value::String(StringValue::new(value, false))
    }

    pub fn block_string(value: impl Into<String>) -> Self {
        Value::String(StringValue::new(value, true))
    }

    pub fn int(value: impl Into<String>) -> Self {
        Value::Int(IntValue { value: value.into() })
    }

    pub fn float(value: impl Into<String>) -> Self {
        Value::Float(FloatValue { value: value.into() })
    }

    pub fn boolean(value: bool) -> Self {
        Value::Boolean(BooleanValue { value })
    }

    pub fn null() -> Self {
        Value::Null(NullValue {})
    }

    pub fn enum_value(value: impl Into<String>) -> Self {
        Value::Enum(EnumValue { value: value.into() })
    }

    pub fn variable(name: impl Into<String>) -> Self {
        Value::Variable(Variable::new(name))
    }

    pub fn list(values: Vec<Value>) -> Self {
        Value::List(ListValue { values })
    }

    pub fn object(fields: Vec<ObjectField>) -> Self {
        Value::Object(ObjectValue { fields })
    }
}
