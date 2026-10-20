//! Port of `__tests__/subgraph/context-reference.spec.ts`.

use federation_composition_rs::subgraph::context::{parse_context_reference, ContextReference};

mod parse_context_reference {
    use super::*;

    /// The body of `parses %j`: `expect(parseContextReference(input)).toEqual({ context, selection })`.
    fn assert_parses(input: &str, context: &str, selection: &str) {
        assert_eq!(
            parse_context_reference(input),
            ContextReference {
                context: Some(context.to_string()),
                selection: Some(selection.to_string())
            }
        );
    }

    /// The body of `rejects %j`: `{ context: undefined, selection: undefined }`.
    fn assert_rejects(input: &str) {
        assert_eq!(
            parse_context_reference(input),
            ContextReference {
                context: None,
                selection: None
            }
        );
    }

    #[test]
    fn parses_review_ctx_sentiment() {
        assert_parses("$reviewCtx { sentiment }", "reviewCtx", " { sentiment }");
    }

    #[test]
    fn parses_review_ctx() {
        assert_parses("$reviewCtx", "reviewCtx", "");
    }

    // GraphQL ignored tokens may surround the `$`
    #[test]
    fn parses_n_review_ctx_id() {
        assert_parses("\n  , $ , reviewCtx { id }", "reviewCtx", " { id }");
    }

    #[test]
    fn parses_pick_the_review_n_review_ctx_id() {
        assert_parses("# pick the review\n$reviewCtx { id }", "reviewCtx", " { id }");
    }

    #[test]
    fn parses_review_ctx_id() {
        assert_parses("\u{feff}$reviewCtx { id }", "reviewCtx", " { id }");
    }

    #[test]
    fn parses_private_id() {
        assert_parses("$_private { id }", "_private", " { id }");
    }

    #[test]
    fn parses_ctx2_id() {
        assert_parses("$ctx2 { id }", "ctx2", " { id }");
    }

    #[test]
    fn rejects_sentiment() {
        assert_rejects("{ sentiment }");
    }

    #[test]
    fn rejects_review_ctx_id() {
        assert_rejects("reviewCtx { id }");
    }

    #[test]
    fn rejects_id() {
        assert_rejects("$ { id }");
    }

    #[test]
    fn rejects_2ctx_id() {
        assert_rejects("$2ctx { id }");
    }

    #[test]
    fn rejects() {
        assert_rejects("");
    }
}
