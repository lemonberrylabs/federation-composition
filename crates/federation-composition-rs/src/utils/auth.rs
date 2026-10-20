//! Port of `src/utils/auth.ts`.

use crate::utils::collections::{IndexMap, IndexSet};

use crate::utils::js::locale_compare;

/// `mergeScopePolicies`: combines two `@requiresScopes`/`@policy` values (OR of AND groups).
/// NOTE(port): the TS sorts the merged groups with `.sort()` (UTF-16 order, byte order for
/// ASCII) and the final list with `localeCompare` (`crate::utils::js::locale_compare`).
pub fn merge_scope_policies(policy_a: &[Vec<String>], policy_b: &[Vec<String>]) -> Vec<Vec<String>> {
    if policy_a.is_empty() || policy_b.is_empty() {
        return if !policy_a.is_empty() {
            policy_a.to_vec()
        } else {
            policy_b.to_vec()
        };
    }

    let mut group_map: IndexMap<String, Vec<String>> = IndexMap::default();

    for group_a in policy_a {
        for group_b in policy_b {
            // Merge by taking union of both groups
            let mut merged: Vec<String> = group_a
                .iter()
                .chain(group_b.iter())
                .cloned()
                .collect::<IndexSet<String>>()
                .into_iter()
                .collect();
            merged.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            let key = merged.join(",");
            group_map.insert(key, merged);
        }
    }

    // Prune redundant groups, keep only minimal groups (subsets eliminate their supersets)
    let mut candidates: Vec<Vec<String>> = group_map.into_values().collect();
    candidates.sort_by_key(|a| a.len());

    let mut final_groups: Vec<Vec<String>> = Vec::new();
    for candidate in candidates {
        if !final_groups.iter().any(|existing| is_subset(existing, &candidate)) {
            final_groups.push(candidate);
        }
    }

    // Final sort by length, then lexicographically
    final_groups.sort_by(|a, b| {
        a.len()
            .cmp(&b.len())
            .then_with(|| locale_compare(&a.join(","), &b.join(",")))
    });

    final_groups
}

fn is_subset(subset: &[String], superset: &[String]) -> bool {
    if subset.len() > superset.len() {
        return false;
    }
    let superset_set: crate::utils::collections::IndexSet<&String> = superset.iter().collect();
    subset.iter().all(|item| superset_set.contains(item))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(groups: &[&[&str]]) -> Vec<Vec<String>> {
        groups
            .iter()
            .map(|g| g.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    /// Expected values generated with the TS function.
    #[test]
    fn merges() {
        let cases: &[(&[&[&str]], &[&[&str]], &[&[&str]])] = &[
            (&[], &[&["b"], &["a"]], &[&["b"], &["a"]]),
            (&[&["a"]], &[], &[&["a"]]),
            (
                &[&["read:x", "B"], &["a"]],
                &[&["b"], &["_z", "a"]],
                &[&["_z", "a"], &["a", "b"], &["B", "b", "read:x"]],
            ),
            (
                &[&["Z"], &["a", "b"]],
                &[&["a"], &["z"], &["A"]],
                &[&["a", "b"], &["A", "Z"], &["Z", "a"], &["Z", "z"]],
            ),
            (&[&["x", "y"]], &[&["y", "x"], &["x"]], &[&["x", "y"]]),
            (&[&["é"], &["e"]], &[&["f"]], &[&["e", "f"], &["f", "é"]]),
        ];
        for (a, b, expected) in cases {
            assert_eq!(merge_scope_policies(&p(a), &p(b)), p(expected), "{a:?} {b:?}");
        }
    }
}
