//! Port of `src/utils/state.ts`.

/// `stripTypeModifiers("[A!]!")` is `"A"`.
pub fn strip_type_modifiers(ty: &str) -> String {
    ty.replace(['!', '[', ']'], "")
}

/// `stripNonNull`: removes one trailing `!`.
pub fn strip_non_null(ty: &str) -> String {
    ty.strip_suffix('!').unwrap_or(ty).to_string()
}

/// `stripList`: removes one leading `[` and one trailing `]`.
pub fn strip_list(ty: &str) -> String {
    let ty = ty.strip_prefix('[').unwrap_or(ty);
    ty.strip_suffix(']').unwrap_or(ty).to_string()
}

pub fn is_non_null(ty: &str) -> bool {
    ty.ends_with('!')
}

pub fn is_list(ty: &str) -> bool {
    ty.ends_with(']')
}
