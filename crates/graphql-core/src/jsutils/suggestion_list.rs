//! graphql-js `jsutils/suggestionList`.

use crate::collections::IndexMap;
use crate::js::js_len;

use super::natural_compare::natural_compare;

/// graphql-js `suggestionList`: the options close to `input` (by
/// Damerau-Levenshtein distance, case-insensitively), sorted by distance and
/// then by [`natural_compare`].
pub fn suggestion_list<S: AsRef<str>>(input: &str, options: &[S]) -> Vec<String> {
    // NOTE(port): a JS object keyed by option; options are GraphQL names,
    // never integer-like, so `Object.keys` is insertion order (a repeated
    // option keeps its first position).
    let mut options_by_distance: IndexMap<String, usize> = IndexMap::default();
    let lexical_distance = LexicalDistance::new(input);
    let threshold = (js_len(input) as f64 * 0.4).floor() as usize + 1;
    for option in options {
        let option = option.as_ref();
        if let Some(distance) = lexical_distance.measure(option, threshold) {
            options_by_distance.insert(option.to_string(), distance);
        }
    }
    let mut keys: Vec<String> = options_by_distance.keys().cloned().collect();
    keys.sort_by(|a, b| {
        let distance_diff = options_by_distance[a] as i64 - options_by_distance[b] as i64;
        if distance_diff != 0 {
            distance_diff.cmp(&0)
        } else {
            natural_compare(a, b)
        }
    });
    keys
}

struct LexicalDistance<'a> {
    input: &'a str,
    input_lower_case: String,
    input_array: Vec<u16>,
}

impl<'a> LexicalDistance<'a> {
    fn new(input: &'a str) -> Self {
        let input_lower_case = input.to_lowercase();
        let input_array = string_to_array(&input_lower_case);
        LexicalDistance {
            input,
            input_lower_case,
            input_array,
        }
    }

    fn measure(&self, option: &str, threshold: usize) -> Option<usize> {
        if self.input == option {
            return Some(0);
        }
        let option_lower_case = option.to_lowercase();
        if self.input_lower_case == option_lower_case {
            return Some(1);
        }
        let mut a = string_to_array(&option_lower_case);
        let mut b = self.input_array.clone();
        if a.len() < b.len() {
            std::mem::swap(&mut a, &mut b);
        }
        let a_length = a.len();
        let b_length = b.len();
        if a_length - b_length > threshold {
            return None;
        }
        // NOTE(port): graphql-js sizes the rows by `input.length + 1` once and
        // reuses them; every cell read below was written for this option.
        let mut rows = vec![vec![0usize; b_length + 1]; 3];
        for (j, cell) in rows[0].iter_mut().enumerate() {
            *cell = j;
        }
        for i in 1..=a_length {
            let up = (i - 1) % 3;
            let current = i % 3;
            rows[current][0] = i;
            let mut smallest_cell = i;
            for j in 1..=b_length {
                let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
                let mut current_cell = (rows[up][j] + 1)
                    .min(rows[current][j - 1] + 1)
                    .min(rows[up][j - 1] + cost);
                if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                    let double_diagonal_cell = rows[(i - 2) % 3][j - 2];
                    current_cell = current_cell.min(double_diagonal_cell + 1);
                }
                if current_cell < smallest_cell {
                    smallest_cell = current_cell;
                }
                rows[current][j] = current_cell;
            }
            if smallest_cell > threshold {
                return None;
            }
        }
        let distance = rows[a_length % 3][b_length];
        if distance <= threshold {
            Some(distance)
        } else {
            None
        }
    }
}

fn string_to_array(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_like_graphql_js() {
        assert_eq!(
            suggestion_list("Strng", &["String", "Int", "Sting", "string"]),
            vec!["Sting", "String", "string"]
        );
        assert!(suggestion_list("abc", &["xyz"]).is_empty());
        assert_eq!(suggestion_list("a", &["a", "A"]), vec!["a", "A"]);
    }
}
