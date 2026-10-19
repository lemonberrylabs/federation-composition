//! Port of `src/utils/version.ts`.

use crate::specifications::federation::FederationVersion;
use crate::utils::js::parse_float;

/// `satisfiesVersionRange(version, range)` where `range` is `"<op> vX.Y"` with `<op>` one of
/// `<`, `>=`, `>`.
/// NOTE(port): both versions go through `parseFloat`, so `v2.10` would compare as `2.1`.
/// Kept as is (behavior).
pub fn satisfies_version_range(version: FederationVersion, range: &str) -> bool {
    let (sign, ver) = range.split_once(' ').unwrap_or((range, ""));
    let version_in_range = parse_float(&ver.replacen('v', "", 1));
    let detected_version = parse_float(&version.as_str().replacen('v', "", 1));

    if sign == "<" {
        return detected_version < version_in_range;
    }

    if sign == ">" {
        return detected_version > version_in_range;
    }

    detected_version >= version_in_range
}
