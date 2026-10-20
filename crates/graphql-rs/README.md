# graphql-rs

A Rust port of [graphql-js](https://github.com/graphql/graphql-js) 17.0.2: the language layer (AST, lexer, parser, printer, visitor, `stripIgnoredCharacters`, `concatAST`) and schema building and validation (`buildSchema`, `buildASTSchema`, `validateSchema`, `assertValidSchema`). Parse results, error messages and printed output match graphql-js byte for byte; the tests in `tests/` compare the two directly.

## Credits

graphql-rs is a port of graphql-js by the GraphQL Contributors, and keeps its MIT license. See [LICENSE](LICENSE).
