//! Port of `src/supergraph/validation/rules/satisfiablity/contexts.ts`.
//!
//! The TS stores context sets as `bigint` bitmasks; [`ContextMask`] is that bit set (it grows
//! beyond 64/128 bits, one bit per distinct context).

use std::cell::RefCell;
use std::fmt;

use indexmap::{IndexMap, IndexSet};

use crate::supergraph::state::SupergraphState;

/// A `bigint` bitmask of contexts. Normalized (no trailing zero words) so `Eq`/`Hash` match
/// bigint equality; `ContextMask::default()` is `0n`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ContextMask {
    words: Vec<u64>,
}

impl ContextMask {
    /// `0n`.
    pub fn zero() -> Self {
        ContextMask::default()
    }

    /// `1n << BigInt(n)`.
    pub fn bit(n: usize) -> Self {
        let mut words = vec![0u64; n / 64 + 1];
        words[n / 64] = 1u64 << (n % 64);
        ContextMask { words }
    }

    /// `mask === 0n`.
    pub fn is_zero(&self) -> bool {
        self.words.is_empty()
    }

    /// `a | b`.
    pub fn or(&self, other: &ContextMask) -> ContextMask {
        let len = self.words.len().max(other.words.len());
        let words = (0..len)
            .map(|i| self.words.get(i).copied().unwrap_or(0) | other.words.get(i).copied().unwrap_or(0))
            .collect();
        ContextMask { words }.normalized()
    }

    /// `a |= b`.
    pub fn or_assign(&mut self, other: &ContextMask) {
        *self = self.or(other);
    }

    /// `a & b`.
    pub fn and(&self, other: &ContextMask) -> ContextMask {
        let len = self.words.len().min(other.words.len());
        let words = (0..len).map(|i| self.words[i] & other.words[i]).collect();
        ContextMask { words }.normalized()
    }

    /// `(self & required) === required`.
    pub fn contains_all(&self, required: &ContextMask) -> bool {
        &self.and(required) == required
    }

    fn normalized(mut self) -> Self {
        while self.words.last() == Some(&0) {
            self.words.pop();
        }
        self
    }
}

impl fmt::Display for ContextMask {
    /// The decimal `bigint` string (`${mask}`), as the TS logs it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.words.is_empty() {
            return f.write_str("0");
        }
        // Repeated division by 10^18 over the little-endian words.
        let mut words = self.words.clone();
        let mut chunks: Vec<u64> = Vec::new();
        const BASE: u128 = 1_000_000_000_000_000_000;
        while !words.is_empty() {
            let mut rem: u128 = 0;
            for w in words.iter_mut().rev() {
                let cur = (rem << 64) | *w as u128;
                *w = (cur / BASE) as u64;
                rem = cur % BASE;
            }
            chunks.push(rem as u64);
            while words.last() == Some(&0) {
                words.pop();
            }
        }
        let mut s = chunks.last().map(|c| c.to_string()).unwrap_or_default();
        for c in chunks.iter().rev().skip(1) {
            s.push_str(&format!("{c:018}"));
        }
        f.write_str(&s)
    }
}

/// `ContextAwareType`: `{ name, contexts, interfaces }` of an object or interface type state.
#[derive(Clone, Copy, Debug)]
pub struct ContextAwareType<'t> {
    pub name: &'t str,
    pub contexts: &'t IndexSet<String>,
    pub interfaces: &'t IndexSet<String>,
}

/// `Contexts`: the contexts available on each type as a bitmask. Bits are assigned lazily the
/// first time a context is seen (also after construction, by `maskOf` calls while graphs are
/// built), so the assignment table is a `RefCell`; the value is shared as `Rc<Contexts>` by the
/// merged graph and the subgraph graphs.
#[derive(Debug, Default)]
pub struct Contexts {
    bit_by_context: RefCell<IndexMap<String, ContextMask>>,
    pub by_type_name: IndexMap<String, ContextMask>,
}

impl Contexts {
    pub fn new(supergraph_state: &SupergraphState) -> Self {
        todo!()
    }

    /// The bit assigned to a single context.
    pub fn mask_of(&self, context: &str) -> ContextMask {
        self.bit_for(context)
    }

    pub fn mask_for<'c>(&self, contexts: impl IntoIterator<Item = &'c String>) -> ContextMask {
        todo!()
    }

    /// The bit assigned to a context; new contexts receive the next unused bit.
    fn bit_for(&self, context: &str) -> ContextMask {
        todo!()
    }

    fn add_mask(&mut self, type_name: &str, mask: ContextMask) {
        todo!()
    }

    fn add_union_contexts(&mut self, supergraph_state: &SupergraphState) {
        todo!()
    }

    /// Adds the contexts declared on the type itself and on the interfaces it implements.
    fn add_type_contexts(&mut self, supergraph_state: &SupergraphState, ty: ContextAwareType<'_>) {
        todo!()
    }
}
