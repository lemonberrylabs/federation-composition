//! Port of `src/supergraph/validation/rules/satisfiablity/moves.ts`.
//!
//! `Move` is an interface with three classes; here it is an enum. `instanceof FieldMove` is
//! `matches!(m, Move::Field(_))`.

use std::fmt;
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
    pub fn as_field(&self) -> Option<&FieldMove> {
        match self {
            Move::Field(m) => Some(m),
            _ => None,
        }
    }
}

impl fmt::Display for Move {
    /// `toString()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Move::Field(m) => m.fmt(f),
            Move::Abstract(m) => m.fmt(f),
            Move::Entity(m) => m.fmt(f),
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
}

impl fmt::Display for FieldMove {
    /// `toString()` (lazy in TS). Used for logging and for the `debugPostFix` of duplicated nodes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.field_name)?;

        if let Some(requires) = &self.requires {
            write!(f, " @require({requires})")?;
        }

        if let Some(provides) = &self.provides {
            write!(f, " @provides({provides})")?;
        }

        if self.provided {
            f.write_str(" @provided")?;
        }

        if let Some(r#override) = &self.r#override {
            // `${null}` is "null".
            write!(f, " @override(label: {}", r#override.label.as_deref().unwrap_or("null"))?;
            write!(f, ", on: {})", r#override.value)?;
        }

        if !self.required_contexts.is_zero() {
            write!(f, " @fromContext(mask: {})", self.required_contexts)?;
        }

        Ok(())
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
}

impl fmt::Display for AbstractMove {
    /// `toString()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.key_fields {
            Some(k) => write!(f, "🔮 🔑 {k}"),
            None => f.write_str("🔮"),
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
}

impl fmt::Display for EntityMove {
    /// `toString()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "🔑 {}", self.key_fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(source: &str) -> Rc<Selection> {
        Rc::new(Selection::new("User".to_string(), source.to_string(), Vec::new()))
    }

    #[test]
    fn field_move_to_string() {
        let plain = FieldMove::new(
            "User".into(),
            "name".into(),
            None,
            None,
            None,
            false,
            ContextMask::zero(),
        );
        assert_eq!(plain.to_string(), "name");

        let full = FieldMove::new(
            "User".into(),
            "name".into(),
            Some(selection("id\n  email")),
            Some(selection("a { b }")),
            Some(FieldMoveOverride {
                label: None,
                from_graph_id: None,
                value: false,
            }),
            true,
            ContextMask::bit(0).or(&ContextMask::bit(2)),
        );
        assert_eq!(
            full.to_string(),
            "name @require(id email) @provides(a { b }) @provided @override(label: null, on: false) @fromContext(mask: 5)"
        );

        let labelled = FieldMove::new(
            "User".into(),
            "name".into(),
            None,
            None,
            Some(FieldMoveOverride {
                label: Some("percent(5)".into()),
                from_graph_id: Some("a".into()),
                value: true,
            }),
            false,
            ContextMask::zero(),
        );
        assert_eq!(labelled.to_string(), "name @override(label: percent(5), on: true)");
    }

    #[test]
    fn abstract_and_entity_move_to_string() {
        assert_eq!(AbstractMove::new(None).to_string(), "🔮");
        assert_eq!(AbstractMove::new(Some(selection("id"))).to_string(), "🔮 🔑 id");
        assert_eq!(EntityMove::new(selection("id  name")).to_string(), "🔑 id name");
    }
}
