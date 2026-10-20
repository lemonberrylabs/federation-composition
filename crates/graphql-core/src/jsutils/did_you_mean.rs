//! graphql-js `jsutils/didYouMean`.

use super::format_list::or_list;

const MAX_SUGGESTIONS: usize = 5;

/// graphql-js `didYouMean(suggestions)` (`sub_message` `None`) and
/// `didYouMean(subMessage, suggestions)`.
pub fn did_you_mean(sub_message: Option<&str>, suggestions: &[String]) -> String {
    if suggestions.is_empty() {
        return String::new();
    }
    let mut message = String::from(" Did you mean ");
    if let Some(sub_message) = sub_message {
        message.push_str(sub_message);
        message.push(' ');
    }
    let quoted: Vec<String> = suggestions
        .iter()
        .take(MAX_SUGGESTIONS)
        .map(|x| format!("\"{x}\""))
        .collect();
    let suggestion_list = or_list(&quoted);
    message + &suggestion_list + "?"
}
