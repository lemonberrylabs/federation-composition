//! Port of `src/supergraph/validation/rules/satisfiablity/moves.ts`.
//!
//! `Move` is an interface with three classes; here it is an enum. `instanceof FieldMove` is
//! `matches!(m, Move::Field(_))`.

use std::rc::Rc;

use super::contexts::ContextMask;
use super::selection::Selection;

/// `Move`.
#[derive(Clone, Debug, PartialEq)]
pub enum Move {
    Field(FieldMove),
    Abstract(AbstractMove),
    Entity(EntityMove),
}

impl Move {
    /// `toString()`.
    pub fn to_string(&self) -> String {
        match self {
            Move::Field(m) => m.to_string(),
            Move::Abstract(m) => m.to_string(),
            Move::Entity(m) => m.to_string(),
        }
    }

    pub fn as_field(&self) -> Option<&FieldMove> {
        match self {
            Move::Field(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_abstract(&self) -> Option<&AbstractMove> {
        match self {
            Move::Abstract(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_entity(&self) -> Option<&EntityMove> {
        match self {
            Move::Entity(m) => Some(m),
            _ => None,
        }
    }
}

/// `FieldMoveOverride` (non-null).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldMoveOverride {
    pub label: Option<String>,
    pub from_graph_id: Option<String>,
    pub value: bool,
}

/// `FieldMove`.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldMove {
    pub type_name: String,
    pub field_name: String,
    pub requires: Option<Rc<Selection>>,
    pub provides: Option<Rc<Selection>>,
    /// `_override` (the `override` getter/setter).
    pub r#override: Option<FieldMoveOverride>,
    pub provided: bool,
    /// Mask of contexts that have to be set by an ancestor for the field's `@fromContext`
    /// arguments to be resolvable.
    pub required_contexts: ContextMask,
}

impl FieldMove {
    pub fn new(
        type_name: String,
        field_name: String,
        requires: Option<Rc<Selection>>,
        provides: Option<Rc<Selection>>,
        r#override: Option<FieldMoveOverride>,
        provided: bool,
        required_contexts: ContextMask,
    ) -> Self {
        FieldMove {
            type_name,
            field_name,
            requires,
            provides,
            r#override,
            provided,
            required_contexts,
        }
    }

    /// `toString()` (lazy in TS; logging only).
    pub fn to_string(&self) -> String {
        todo!()
    }
}

/// `AbstractMove`.
#[derive(Clone, Debug, PartialEq)]
pub struct AbstractMove {
    pub key_fields: Option<Rc<Selection>>,
}

impl AbstractMove {
    pub fn new(key_fields: Option<Rc<Selection>>) -> Self {
        AbstractMove { key_fields }
    }

    pub fn to_string(&self) -> String {
        match &self.key_fields {
            Some(k) => format!("🔮 🔑 {}", k.to_string()),
            None => "🔮".to_string(),
        }
    }
}

/// `EntityMove`.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityMove {
    pub key_fields: Rc<Selection>,
}

impl EntityMove {
    pub fn new(key_fields: Rc<Selection>) -> Self {
        EntityMove { key_fields }
    }

    pub fn to_string(&self) -> String {
        format!("🔑 {}", self.key_fields.to_string())
    }
}
