//! graphql-js `jsutils/naturalCompare`.

use std::cmp::Ordering;

const DIGIT_0: u16 = 48;
const DIGIT_9: u16 = 57;

/// graphql-js `naturalCompare`: compares UTF-16 code units, and runs of
/// digits by their numeric value.
pub fn natural_compare(a_str: &str, b_str: &str) -> Ordering {
    let a: Vec<u16> = a_str.encode_utf16().collect();
    let b: Vec<u16> = b_str.encode_utf16().collect();
    let mut a_index = 0;
    let mut b_index = 0;
    while a_index < a.len() && b_index < b.len() {
        // `charCodeAt` past the end is NaN: `None`.
        let mut a_char = a.get(a_index).copied();
        let mut b_char = b.get(b_index).copied();
        if is_digit(a_char) && is_digit(b_char) {
            let mut a_num = 0f64;
            loop {
                a_index += 1;
                a_num = a_num * 10.0 + f64::from(a_char.unwrap() - DIGIT_0);
                a_char = a.get(a_index).copied();
                if !(is_digit(a_char) && a_num > 0.0) {
                    break;
                }
            }
            let mut b_num = 0f64;
            loop {
                b_index += 1;
                b_num = b_num * 10.0 + f64::from(b_char.unwrap() - DIGIT_0);
                b_char = b.get(b_index).copied();
                if !(is_digit(b_char) && b_num > 0.0) {
                    break;
                }
            }
            if a_num < b_num {
                return Ordering::Less;
            }
            if a_num > b_num {
                return Ordering::Greater;
            }
        } else {
            if a_char < b_char {
                return Ordering::Less;
            }
            if a_char > b_char {
                return Ordering::Greater;
            }
            a_index += 1;
            b_index += 1;
        }
    }
    a.len().cmp(&b.len())
}

fn is_digit(code: Option<u16>) -> bool {
    matches!(code, Some(c) if (DIGIT_0..=DIGIT_9).contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_like_graphql_js() {
        assert_eq!(natural_compare("a2", "a10"), Ordering::Less);
        assert_eq!(natural_compare("abc", "abc"), Ordering::Equal);
        assert_eq!(natural_compare("ab", "abc"), Ordering::Less);
        assert_eq!(natural_compare("B", "a"), Ordering::Less);
        assert_eq!(natural_compare("a01", "a1"), Ordering::Less);
    }
}
