//! Port of `src/supergraph/validation/rules/satisfiablity/contexts.ts`.
//!
//! The TS stores context sets as `bigint` bitmasks; [`ContextMask`] is that bit set (it grows
//! beyond 64/128 bits, one bit per distinct context).

use std::cell::RefCell;
use std::fmt;

use crate::utils::collections::{IndexMap, IndexSet};

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
        let mut contexts = Contexts::default();
        // Union contexts are processed first because union members
        // inherit the contexts declared on their unions.
        contexts.add_union_contexts(supergraph_state);

        for ty in supergraph_state.object_types.values() {
            contexts.add_type_contexts(
                supergraph_state,
                ContextAwareType {
                    name: &ty.name,
                    contexts: &ty.contexts,
                    interfaces: &ty.interfaces,
                },
            );
        }

        for ty in supergraph_state.interface_types.values() {
            contexts.add_type_contexts(
                supergraph_state,
                ContextAwareType {
                    name: &ty.name,
                    contexts: &ty.contexts,
                    interfaces: &ty.interfaces,
                },
            );
        }

        contexts
    }

    /// The bit assigned to a single context.
    pub fn mask_of(&self, context: &str) -> ContextMask {
        self.bit_for(context)
    }

    pub fn mask_for<'c>(&self, contexts: impl IntoIterator<Item = &'c String>) -> ContextMask {
        // no contexts by default
        let mut mask = ContextMask::zero();

        for context in contexts {
            mask.or_assign(&self.bit_for(context));
        }

        mask
    }

    /// The bit assigned to a context; new contexts receive the next unused bit.
    fn bit_for(&self, context: &str) -> ContextMask {
        if let Some(bit) = self.bit_by_context.borrow().get(context) {
            return bit.clone();
        }

        let mut bit_by_context = self.bit_by_context.borrow_mut();
        let bit = ContextMask::bit(bit_by_context.len());
        bit_by_context.insert(context.to_string(), bit.clone());
        bit
    }

    fn add_mask(&mut self, type_name: &str, mask: ContextMask) {
        if mask.is_zero() {
            return;
        }

        let existing = self.by_type_name.get(type_name).cloned().unwrap_or_default();
        self.by_type_name.insert(type_name.to_string(), existing.or(&mask));
    }

    fn add_union_contexts(&mut self, supergraph_state: &SupergraphState) {
        for union in supergraph_state.union_types.values() {
            let mask = self.mask_for(&union.contexts);

            for member in &union.members {
                self.add_mask(member, mask.clone());
            }
        }
    }

    /// Adds the contexts declared on the type itself and on the interfaces it implements.
    fn add_type_contexts(&mut self, supergraph_state: &SupergraphState, ty: ContextAwareType<'_>) {
        let mut mask = self.mask_for(ty.contexts);

        // The list already has all parents, so one look is enough.
        for interface_name in ty.interfaces {
            if let Some(interface_type) = supergraph_state.interface_types.get(interface_name) {
                mask.or_assign(&self.mask_for(&interface_type.contexts));
            }
        }

        self.add_mask(ty.name, mask);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_are_assigned_in_first_seen_order() {
        let contexts = Contexts::default();
        assert_eq!(contexts.mask_of("auth"), ContextMask::bit(0));
        assert_eq!(contexts.mask_of("tenant"), ContextMask::bit(1));
        assert_eq!(contexts.mask_of("auth"), ContextMask::bit(0));
        let both = contexts.mask_for(&["tenant".to_string(), "locale".to_string()]);
        assert_eq!(both, ContextMask::bit(1).or(&ContextMask::bit(2)));
        assert_eq!(both.to_string(), "6");
    }

    #[test]
    fn masks_grow_beyond_64_bits() {
        let contexts = Contexts::default();
        for i in 0..130 {
            contexts.mask_of(&format!("c{i}"));
        }
        let high = contexts.mask_of("c129");
        assert_eq!(high, ContextMask::bit(129));
        // 2^129
        assert_eq!(high.to_string(), "680564733841876926926749214863536422912");
        let low = contexts.mask_of("c0");
        assert!(!high.or(&low).contains_all(&ContextMask::bit(5)));
        assert!(high.or(&low).contains_all(&low));
        assert!(high.and(&low).is_zero());
        assert!(ContextMask::zero().contains_all(&ContextMask::zero()));
    }
}
