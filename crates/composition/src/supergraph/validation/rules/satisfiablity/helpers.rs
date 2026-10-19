//! Port of `src/supergraph/validation/rules/satisfiablity/helpers.ts`.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;

/// `occurrences(str, subString)`.
/// NOTE(port): lengths are UTF-16 in JS; key fields are ASCII.
pub fn occurrences(s: &str, sub_string: &str) -> usize {
    if sub_string.is_empty() {
        return s.len() + 1;
    }
    s.matches(sub_string).count()
}

/// `scoreKeyFields(keyFields)`.
pub fn score_key_fields(key_fields: &str) -> usize {
    let fields = occurrences(key_fields, " ") + 1;
    let inner_selection_sets = occurrences(key_fields, "{") * 3;
    fields + inner_selection_sets
}

/// `Lazy<T>` / `lazy(factory)`: a memoized value computed on the first `get()`.
///
/// The factory may borrow data living for `'a` (e.g. the satisfiability graphs, which are
/// `Rc`-shared). Share one `Lazy` between owners with `Rc<Lazy<..>>` (the TS shares the object).
/// The `toString` caches of `Node`/`Edge`/moves/`OperationPath` are only used for logging and
/// can be computed on demand instead.
pub struct Lazy<'a, T> {
    factory: Rc<dyn Fn() -> T + 'a>,
    value: RefCell<Option<T>>,
}

/// `lazy(factory)`.
pub fn lazy<'a, T>(factory: impl Fn() -> T + 'a) -> Lazy<'a, T> {
    Lazy {
        factory: Rc::new(factory),
        value: RefCell::new(None),
    }
}

impl<T: Clone> Lazy<'_, T> {
    pub fn get(&self) -> T {
        if let Some(v) = self.value.borrow().as_ref() {
            return v.clone();
        }
        let v = (self.factory)();
        *self.value.borrow_mut() = Some(v.clone());
        v
    }

    pub fn invalidate(&self) {
        *self.value.borrow_mut() = None;
    }
}

impl<T> std::fmt::Debug for Lazy<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lazy")
    }
}

/// `OverrideLabels`: label name to on/off.
/// NOTE(port): a plain object in TS iterated with `for..in` in `matches`; labels are user
/// strings that could be integer-like (ordering only affects iteration, not the result of
/// `matches`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverrideLabels {
    state: IndexMap<String, bool>,
}

impl OverrideLabels {
    pub fn new(state: Option<IndexMap<String, bool>>) -> Self {
        OverrideLabels {
            state: state.unwrap_or_default(),
        }
    }

    pub fn set(mut self, key: &str, value: bool) -> Self {
        self.state.insert(key.to_string(), value);
        self
    }

    pub fn get(&self, key: &str) -> Option<bool> {
        self.state.get(key).copied()
    }

    /// Every label set here has the same value in `other` (`undefined !== true/false`).
    pub fn matches(&self, other: &OverrideLabels) -> bool {
        self.state.iter().all(|(k, v)| other.state.get(k) == Some(v))
    }

    pub fn clone_labels(&self) -> OverrideLabels {
        self.clone()
    }
}
