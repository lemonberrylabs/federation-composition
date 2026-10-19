//! Port of `src/utils/auth.ts`.

/// `mergeScopePolicies`: combines two `@requiresScopes`/`@policy` values (OR of AND groups).
/// NOTE(port): the TS sorts the merged groups with `.sort()` (UTF-16 order, byte order for
/// ASCII) and the final list with `localeCompare` (`crate::utils::js::locale_compare`).
pub fn merge_scope_policies(policy_a: &[Vec<String>], policy_b: &[Vec<String>]) -> Vec<Vec<String>> {
    todo!()
}

fn is_subset(subset: &[String], superset: &[String]) -> bool {
    if subset.len() > superset.len() {
        return false;
    }
    let superset_set: indexmap::IndexSet<&String> = superset.iter().collect();
    subset.iter().all(|item| superset_set.contains(item))
}
