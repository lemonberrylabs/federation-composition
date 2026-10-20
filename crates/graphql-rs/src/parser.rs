//! Port of graphql-js 17.0.2 `language/parser`: a recursive-descent parser
//! with graphql-js's grammar and error messages.

use std::borrow::Cow;

use crate::ast::*;
use crate::directive_location::DirectiveLocation;
use crate::error::GraphQLError;
use crate::lexer::{token_desc, token_kind_desc, Lexer, Token, TokenKind};

/// The graphql-js `ParseOptions` that affect the produced AST or errors.
#[derive(Clone, Debug, Default)]
pub struct ParseOptions {
    /// graphql-js `maxTokens`.
    pub max_tokens: Option<usize>,
    /// graphql-js `experimentalFragmentArguments`.
    pub experimental_fragment_arguments: bool,
}

type PResult<T> = Result<T, Box<GraphQLError>>;

/// graphql-js `parse`.
pub fn parse(source: &str) -> Result<Document, GraphQLError> {
    parse_with_options(source, ParseOptions::default())
}

/// graphql-js `parse` with options.
pub fn parse_with_options(source: &str, options: ParseOptions) -> Result<Document, GraphQLError> {
    Parser::new(source, options)
        .parse_document()
        .map_err(|e| *e)
}

/// graphql-js `parseValue`.
pub fn parse_value(source: &str) -> Result<Value, GraphQLError> {
    let mut parser = Parser::new(source, ParseOptions::default());
    parser.parse_whole(|p| p.parse_value_literal(false))
}

/// graphql-js `parseConstValue`.
pub fn parse_const_value(source: &str) -> Result<ConstValue, GraphQLError> {
    let mut parser = Parser::new(source, ParseOptions::default());
    parser.parse_whole(|p| p.parse_value_literal(true))
}

/// graphql-js `parseType`.
pub fn parse_type(source: &str) -> Result<Type, GraphQLError> {
    let mut parser = Parser::new(source, ParseOptions::default());
    parser.parse_whole(Parser::parse_type_reference)
}

/// graphql-js `parseSchemaCoordinate`.
pub fn parse_schema_coordinate(source: &str) -> Result<SchemaCoordinate, GraphQLError> {
    let mut parser = Parser {
        lexer: Lexer::new_coordinate(source),
        options: ParseOptions::default(),
        token_counter: 0,
    };
    parser.parse_whole(Parser::parse_schema_coordinate)
}

/// graphql-js `Parser`.
pub(crate) struct Parser<'s> {
    lexer: Lexer<'s>,
    options: ParseOptions,
    token_counter: usize,
}

impl<'s> Parser<'s> {
    fn new(source: &'s str, options: ParseOptions) -> Self {
        Parser {
            lexer: Lexer::new(source),
            options,
            token_counter: 0,
        }
    }

    /// `<SOF>`, the production, `<EOF>`: how graphql-js parses a value, a
    /// type or a schema coordinate.
    fn parse_whole<T>(
        &mut self,
        production: impl FnOnce(&mut Self) -> PResult<T>,
    ) -> Result<T, GraphQLError> {
        (|| {
            self.expect_token(TokenKind::Sof)?;
            let result = production(self)?;
            self.expect_token(TokenKind::Eof)?;
            Ok(result)
        })()
        .map_err(|e: Box<GraphQLError>| *e)
    }

    fn parse_name(&mut self) -> PResult<Name> {
        let token = self.expect_token(TokenKind::Name)?;
        Ok(Name {
            value: token_value(token),
        })
    }

    fn parse_document(&mut self) -> PResult<Document> {
        Ok(Document {
            definitions: self.many(TokenKind::Sof, Self::parse_definition, TokenKind::Eof)?,
        })
    }

    fn parse_definition(&mut self) -> PResult<Definition> {
        if self.peek(TokenKind::BraceL) {
            return Ok(Definition::OperationDefinition(
                self.parse_operation_definition()?,
            ));
        }
        let has_description = self.peek_description();
        let (keyword_kind, keyword_value) = if has_description {
            let t = self.lexer.lookahead()?;
            (t.kind, t.value.clone())
        } else {
            (self.lexer.token.kind, self.lexer.token.value.clone())
        };
        if has_description && keyword_kind == TokenKind::BraceL {
            return Err(GraphQLError::syntax(
                "Unexpected description, descriptions are not supported on shorthand queries.",
            ));
        }
        if keyword_kind == TokenKind::Name {
            match keyword_value.as_deref() {
                Some("schema") => {
                    return Ok(Definition::SchemaDefinition(
                        self.parse_schema_definition()?,
                    ))
                }
                Some("scalar") => {
                    return Ok(Definition::ScalarTypeDefinition(
                        self.parse_scalar_type_definition()?,
                    ))
                }
                Some("type") => {
                    return Ok(Definition::ObjectTypeDefinition(
                        self.parse_object_type_definition()?,
                    ))
                }
                Some("interface") => {
                    return Ok(Definition::InterfaceTypeDefinition(
                        self.parse_interface_type_definition()?,
                    ))
                }
                Some("union") => {
                    return Ok(Definition::UnionTypeDefinition(
                        self.parse_union_type_definition()?,
                    ))
                }
                Some("enum") => {
                    return Ok(Definition::EnumTypeDefinition(
                        self.parse_enum_type_definition()?,
                    ))
                }
                Some("input") => {
                    return Ok(Definition::InputObjectTypeDefinition(
                        self.parse_input_object_type_definition()?,
                    ))
                }
                Some("directive") => {
                    return Ok(Definition::DirectiveDefinition(
                        self.parse_directive_definition()?,
                    ))
                }
                _ => {}
            }
            match keyword_value.as_deref() {
                Some("query") | Some("mutation") | Some("subscription") => {
                    return Ok(Definition::OperationDefinition(
                        self.parse_operation_definition()?,
                    ))
                }
                Some("fragment") => {
                    return Ok(Definition::FragmentDefinition(
                        self.parse_fragment_definition()?,
                    ))
                }
                _ => {}
            }
            if has_description {
                return Err(GraphQLError::syntax(
                    "Unexpected description, only GraphQL definitions support descriptions.",
                ));
            }
            if keyword_value.as_deref() == Some("extend") {
                return self.parse_type_system_extension();
            }
        }
        Err(unexpected_desc(keyword_kind, keyword_value.as_deref()))
    }

    fn parse_operation_definition(&mut self) -> PResult<OperationDefinition> {
        if self.peek(TokenKind::BraceL) {
            return Ok(OperationDefinition {
                operation: OperationType::Query,
                description: None,
                name: None,
                variable_definitions: Vec::new(),
                directives: Vec::new(),
                selection_set: self.parse_selection_set()?,
            });
        }
        let description = self.parse_description()?;
        let operation = self.parse_operation_type()?;
        let mut name = None;
        if self.peek(TokenKind::Name) {
            name = Some(self.parse_name()?);
        }
        Ok(OperationDefinition {
            operation,
            description,
            name,
            variable_definitions: self.parse_variable_definitions()?,
            directives: self.parse_directives(false)?,
            selection_set: self.parse_selection_set()?,
        })
    }

    fn parse_operation_type(&mut self) -> PResult<OperationType> {
        let operation_token = self.expect_token(TokenKind::Name)?;
        match operation_token.value.as_deref() {
            Some("query") => Ok(OperationType::Query),
            Some("mutation") => Ok(OperationType::Mutation),
            Some("subscription") => Ok(OperationType::Subscription),
            _ => Err(unexpected(&operation_token)),
        }
    }

    fn parse_variable_definitions(&mut self) -> PResult<Vec<VariableDefinition>> {
        self.optional_many(
            TokenKind::ParenL,
            Self::parse_variable_definition,
            TokenKind::ParenR,
        )
    }

    fn parse_variable_definition(&mut self) -> PResult<VariableDefinition> {
        let description = self.parse_description()?;
        let variable = self.parse_variable()?;
        self.expect_token(TokenKind::Colon)?;
        let ty = self.parse_type_reference()?;
        let default_value = if self.expect_optional_token(TokenKind::Equals)? {
            Some(self.parse_value_literal(true)?)
        } else {
            None
        };
        Ok(VariableDefinition {
            description,
            variable,
            ty,
            default_value,
            directives: self.parse_directives(true)?,
        })
    }

    fn parse_variable(&mut self) -> PResult<Variable> {
        self.expect_token(TokenKind::Dollar)?;
        Ok(Variable {
            name: self.parse_name()?,
        })
    }

    fn parse_selection_set(&mut self) -> PResult<SelectionSet> {
        Ok(SelectionSet {
            selections: self.many(TokenKind::BraceL, Self::parse_selection, TokenKind::BraceR)?,
        })
    }

    fn parse_selection(&mut self) -> PResult<Selection> {
        if self.peek(TokenKind::Spread) {
            self.parse_fragment()
        } else {
            Ok(Selection::Field(self.parse_field()?))
        }
    }

    fn parse_field(&mut self) -> PResult<Field> {
        let name_or_alias = self.parse_name()?;
        let (alias, name) = if self.expect_optional_token(TokenKind::Colon)? {
            (Some(name_or_alias), self.parse_name()?)
        } else {
            (None, name_or_alias)
        };
        Ok(Field {
            alias,
            name,
            arguments: self.parse_arguments(false)?,
            directives: self.parse_directives(false)?,
            selection_set: if self.peek(TokenKind::BraceL) {
                Some(self.parse_selection_set()?)
            } else {
                None
            },
        })
    }

    fn parse_arguments(&mut self, is_const: bool) -> PResult<Vec<Argument>> {
        let item = if is_const {
            Self::parse_const_argument
        } else {
            Self::parse_non_const_argument
        };
        self.optional_many(TokenKind::ParenL, item, TokenKind::ParenR)
    }

    fn parse_fragment_arguments(&mut self) -> PResult<Vec<FragmentArgument>> {
        self.optional_many(
            TokenKind::ParenL,
            Self::parse_fragment_argument,
            TokenKind::ParenR,
        )
    }

    fn parse_argument(&mut self, is_const: bool) -> PResult<Argument> {
        let name = self.parse_name()?;
        self.expect_token(TokenKind::Colon)?;
        Ok(Argument {
            name,
            value: self.parse_value_literal(is_const)?,
        })
    }

    fn parse_non_const_argument(&mut self) -> PResult<Argument> {
        self.parse_argument(false)
    }

    fn parse_const_argument(&mut self) -> PResult<Argument> {
        self.parse_argument(true)
    }

    fn parse_fragment_argument(&mut self) -> PResult<FragmentArgument> {
        let name = self.parse_name()?;
        self.expect_token(TokenKind::Colon)?;
        Ok(FragmentArgument {
            name,
            value: self.parse_value_literal(false)?,
        })
    }

    fn parse_fragment(&mut self) -> PResult<Selection> {
        self.expect_token(TokenKind::Spread)?;
        let has_type_condition = self.expect_optional_keyword("on")?;
        if !has_type_condition && self.peek(TokenKind::Name) {
            let name = self.parse_fragment_name()?;
            if self.peek(TokenKind::ParenL) && self.options.experimental_fragment_arguments {
                return Ok(Selection::FragmentSpread(FragmentSpread {
                    name,
                    arguments: self.parse_fragment_arguments()?,
                    directives: self.parse_directives(false)?,
                }));
            }
            return Ok(Selection::FragmentSpread(FragmentSpread {
                name,
                arguments: Vec::new(),
                directives: self.parse_directives(false)?,
            }));
        }
        Ok(Selection::InlineFragment(InlineFragment {
            type_condition: if has_type_condition {
                Some(self.parse_named_type()?)
            } else {
                None
            },
            directives: self.parse_directives(false)?,
            selection_set: self.parse_selection_set()?,
        }))
    }

    fn parse_fragment_definition(&mut self) -> PResult<FragmentDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("fragment")?;
        let name = self.parse_fragment_name()?;
        let variable_definitions = if self.options.experimental_fragment_arguments {
            self.parse_variable_definitions()?
        } else {
            Vec::new()
        };
        self.expect_keyword("on")?;
        let type_condition = self.parse_named_type()?;
        Ok(FragmentDefinition {
            description,
            name,
            variable_definitions,
            type_condition,
            directives: self.parse_directives(false)?,
            selection_set: self.parse_selection_set()?,
        })
    }

    fn parse_fragment_name(&mut self) -> PResult<Name> {
        if self.lexer.token.value.as_deref() == Some("on") {
            return Err(unexpected(&self.lexer.token));
        }
        self.parse_name()
    }

    fn parse_value_literal(&mut self, is_const: bool) -> PResult<Value> {
        match self.lexer.token.kind {
            TokenKind::BracketL => self.parse_list(is_const),
            TokenKind::BraceL => self.parse_object(is_const),
            TokenKind::Int => {
                let token = self.advance_lexer()?;
                Ok(Value::Int(IntValue {
                    value: token_value(token),
                }))
            }
            TokenKind::Float => {
                let token = self.advance_lexer()?;
                Ok(Value::Float(FloatValue {
                    value: token_value(token),
                }))
            }
            TokenKind::String | TokenKind::BlockString => {
                Ok(Value::String(self.parse_string_literal()?))
            }
            TokenKind::Name => {
                let token = self.advance_lexer()?;
                Ok(match token.value.as_deref() {
                    Some("true") => Value::Boolean(BooleanValue { value: true }),
                    Some("false") => Value::Boolean(BooleanValue { value: false }),
                    Some("null") => Value::Null(NullValue {}),
                    _ => Value::Enum(EnumValue {
                        value: token_value(token),
                    }),
                })
            }
            TokenKind::Dollar => {
                if is_const {
                    let token = self.expect_token(TokenKind::Dollar)?;
                    if self.lexer.token.kind == TokenKind::Name {
                        let var_name = self.lexer.token.value.as_deref().unwrap_or("");
                        return Err(GraphQLError::syntax(format!(
                            "Unexpected variable \"${var_name}\" in constant value."
                        )));
                    } else {
                        return Err(unexpected(&token));
                    }
                }
                Ok(Value::Variable(self.parse_variable()?))
            }
            _ => Err(unexpected(&self.lexer.token)),
        }
    }

    fn parse_string_literal(&mut self) -> PResult<StringValue> {
        let token = self.advance_lexer()?;
        let block = token.kind == TokenKind::BlockString;
        Ok(StringValue {
            value: token_value(token),
            block,
        })
    }

    fn parse_list(&mut self, is_const: bool) -> PResult<Value> {
        self.expect_token(TokenKind::BracketL)?;
        let mut values = Vec::new();
        while !self.expect_optional_token(TokenKind::BracketR)? {
            values.push(self.parse_value_literal(is_const)?);
        }
        Ok(Value::List(ListValue { values }))
    }

    fn parse_object(&mut self, is_const: bool) -> PResult<Value> {
        self.expect_token(TokenKind::BraceL)?;
        let mut fields = Vec::new();
        while !self.expect_optional_token(TokenKind::BraceR)? {
            fields.push(self.parse_object_field(is_const)?);
        }
        Ok(Value::Object(ObjectValue { fields }))
    }

    fn parse_object_field(&mut self, is_const: bool) -> PResult<ObjectField> {
        let name = self.parse_name()?;
        self.expect_token(TokenKind::Colon)?;
        Ok(ObjectField {
            name,
            value: self.parse_value_literal(is_const)?,
        })
    }

    fn parse_directives(&mut self, is_const: bool) -> PResult<Vec<Directive>> {
        let mut directives = Vec::new();
        while self.peek(TokenKind::At) {
            directives.push(self.parse_directive(is_const)?);
        }
        Ok(directives)
    }

    fn parse_const_directives(&mut self) -> PResult<Vec<Directive>> {
        self.parse_directives(true)
    }

    fn parse_directive(&mut self, is_const: bool) -> PResult<Directive> {
        self.expect_token(TokenKind::At)?;
        Ok(Directive {
            name: self.parse_name()?,
            arguments: self.parse_arguments(is_const)?,
        })
    }

    fn parse_type_reference(&mut self) -> PResult<Type> {
        let ty = if self.expect_optional_token(TokenKind::BracketL)? {
            let inner_type = self.parse_type_reference()?;
            self.expect_token(TokenKind::BracketR)?;
            Type::List(ListType {
                ty: Box::new(inner_type),
            })
        } else {
            Type::Named(self.parse_named_type()?)
        };
        if self.expect_optional_token(TokenKind::Bang)? {
            return Ok(Type::NonNull(NonNullType { ty: Box::new(ty) }));
        }
        Ok(ty)
    }

    fn parse_named_type(&mut self) -> PResult<NamedType> {
        Ok(NamedType {
            name: self.parse_name()?,
        })
    }

    fn peek_description(&self) -> bool {
        self.peek(TokenKind::String) || self.peek(TokenKind::BlockString)
    }

    fn parse_description(&mut self) -> PResult<Option<StringValue>> {
        if self.peek_description() {
            Ok(Some(self.parse_string_literal()?))
        } else {
            Ok(None)
        }
    }

    fn parse_schema_definition(&mut self) -> PResult<SchemaDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("schema")?;
        let directives = self.parse_const_directives()?;
        let operation_types = self.many(
            TokenKind::BraceL,
            Self::parse_operation_type_definition,
            TokenKind::BraceR,
        )?;
        Ok(SchemaDefinition {
            description,
            directives,
            operation_types,
        })
    }

    fn parse_operation_type_definition(&mut self) -> PResult<OperationTypeDefinition> {
        let operation = self.parse_operation_type()?;
        self.expect_token(TokenKind::Colon)?;
        let ty = self.parse_named_type()?;
        Ok(OperationTypeDefinition { operation, ty })
    }

    fn parse_scalar_type_definition(&mut self) -> PResult<ScalarTypeDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("scalar")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        Ok(ScalarTypeDefinition {
            description,
            name,
            directives,
        })
    }

    fn parse_object_type_definition(&mut self) -> PResult<ObjectTypeDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("type")?;
        let name = self.parse_name()?;
        let interfaces = self.parse_implements_interfaces()?;
        let directives = self.parse_const_directives()?;
        let fields = self.parse_fields_definition()?;
        Ok(ObjectTypeDefinition {
            description,
            name,
            interfaces,
            directives,
            fields,
        })
    }

    fn parse_implements_interfaces(&mut self) -> PResult<Vec<NamedType>> {
        if self.expect_optional_keyword("implements")? {
            self.delimited_many(TokenKind::Amp, Self::parse_named_type)
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_fields_definition(&mut self) -> PResult<Vec<FieldDefinition>> {
        self.optional_many(
            TokenKind::BraceL,
            Self::parse_field_definition,
            TokenKind::BraceR,
        )
    }

    fn parse_field_definition(&mut self) -> PResult<FieldDefinition> {
        let description = self.parse_description()?;
        let name = self.parse_name()?;
        let arguments = self.parse_argument_defs()?;
        self.expect_token(TokenKind::Colon)?;
        let ty = self.parse_type_reference()?;
        let directives = self.parse_const_directives()?;
        Ok(FieldDefinition {
            description,
            name,
            arguments,
            ty,
            directives,
        })
    }

    fn parse_argument_defs(&mut self) -> PResult<Vec<InputValueDefinition>> {
        self.optional_many(
            TokenKind::ParenL,
            Self::parse_input_value_def,
            TokenKind::ParenR,
        )
    }

    fn parse_input_value_def(&mut self) -> PResult<InputValueDefinition> {
        let description = self.parse_description()?;
        let name = self.parse_name()?;
        self.expect_token(TokenKind::Colon)?;
        let ty = self.parse_type_reference()?;
        let mut default_value = None;
        if self.expect_optional_token(TokenKind::Equals)? {
            default_value = Some(self.parse_value_literal(true)?);
        }
        let directives = self.parse_const_directives()?;
        Ok(InputValueDefinition {
            description,
            name,
            ty,
            default_value,
            directives,
        })
    }

    fn parse_interface_type_definition(&mut self) -> PResult<InterfaceTypeDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("interface")?;
        let name = self.parse_name()?;
        let interfaces = self.parse_implements_interfaces()?;
        let directives = self.parse_const_directives()?;
        let fields = self.parse_fields_definition()?;
        Ok(InterfaceTypeDefinition {
            description,
            name,
            interfaces,
            directives,
            fields,
        })
    }

    fn parse_union_type_definition(&mut self) -> PResult<UnionTypeDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("union")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        let types = self.parse_union_member_types()?;
        Ok(UnionTypeDefinition {
            description,
            name,
            directives,
            types,
        })
    }

    fn parse_union_member_types(&mut self) -> PResult<Vec<NamedType>> {
        if self.expect_optional_token(TokenKind::Equals)? {
            self.delimited_many(TokenKind::Pipe, Self::parse_named_type)
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_enum_type_definition(&mut self) -> PResult<EnumTypeDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("enum")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        let values = self.parse_enum_values_definition()?;
        Ok(EnumTypeDefinition {
            description,
            name,
            directives,
            values,
        })
    }

    fn parse_enum_values_definition(&mut self) -> PResult<Vec<EnumValueDefinition>> {
        self.optional_many(
            TokenKind::BraceL,
            Self::parse_enum_value_definition,
            TokenKind::BraceR,
        )
    }

    fn parse_enum_value_definition(&mut self) -> PResult<EnumValueDefinition> {
        let description = self.parse_description()?;
        let name = self.parse_enum_value_name()?;
        let directives = self.parse_const_directives()?;
        Ok(EnumValueDefinition {
            description,
            name,
            directives,
        })
    }

    fn parse_enum_value_name(&mut self) -> PResult<Name> {
        if matches!(
            self.lexer.token.value.as_deref(),
            Some("true") | Some("false") | Some("null")
        ) {
            return Err(GraphQLError::syntax(format!(
                "{} is reserved and cannot be used for an enum value.",
                self.lexer.token.desc()
            )));
        }
        self.parse_name()
    }

    fn parse_input_object_type_definition(&mut self) -> PResult<InputObjectTypeDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("input")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        let fields = self.parse_input_fields_definition()?;
        Ok(InputObjectTypeDefinition {
            description,
            name,
            directives,
            fields,
        })
    }

    fn parse_input_fields_definition(&mut self) -> PResult<Vec<InputValueDefinition>> {
        self.optional_many(
            TokenKind::BraceL,
            Self::parse_input_value_def,
            TokenKind::BraceR,
        )
    }

    fn parse_type_system_extension(&mut self) -> PResult<Definition> {
        let keyword_token = self.lexer.lookahead()?;
        let (kind, value) = (keyword_token.kind, keyword_token.value.clone());
        if kind == TokenKind::Name {
            match value.as_deref() {
                Some("schema") => {
                    return Ok(Definition::SchemaExtension(self.parse_schema_extension()?))
                }
                Some("scalar") => {
                    return Ok(Definition::ScalarTypeExtension(
                        self.parse_scalar_type_extension()?,
                    ))
                }
                Some("type") => {
                    return Ok(Definition::ObjectTypeExtension(
                        self.parse_object_type_extension()?,
                    ))
                }
                Some("interface") => {
                    return Ok(Definition::InterfaceTypeExtension(
                        self.parse_interface_type_extension()?,
                    ))
                }
                Some("union") => {
                    return Ok(Definition::UnionTypeExtension(
                        self.parse_union_type_extension()?,
                    ))
                }
                Some("enum") => {
                    return Ok(Definition::EnumTypeExtension(
                        self.parse_enum_type_extension()?,
                    ))
                }
                Some("input") => {
                    return Ok(Definition::InputObjectTypeExtension(
                        self.parse_input_object_type_extension()?,
                    ))
                }
                Some("directive") => {
                    return Ok(Definition::DirectiveExtension(
                        self.parse_directive_extension()?,
                    ))
                }
                _ => {}
            }
        }
        Err(unexpected_desc(kind, value.as_deref()))
    }

    fn parse_schema_extension(&mut self) -> PResult<SchemaExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("schema")?;
        let directives = self.parse_const_directives()?;
        let operation_types = self.optional_many(
            TokenKind::BraceL,
            Self::parse_operation_type_definition,
            TokenKind::BraceR,
        )?;
        if directives.is_empty() && operation_types.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(SchemaExtension {
            directives,
            operation_types,
        })
    }

    fn parse_scalar_type_extension(&mut self) -> PResult<ScalarTypeExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("scalar")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        if directives.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(ScalarTypeExtension { name, directives })
    }

    fn parse_object_type_extension(&mut self) -> PResult<ObjectTypeExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("type")?;
        let name = self.parse_name()?;
        let interfaces = self.parse_implements_interfaces()?;
        let directives = self.parse_const_directives()?;
        let fields = self.parse_fields_definition()?;
        if interfaces.is_empty() && directives.is_empty() && fields.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(ObjectTypeExtension {
            name,
            interfaces,
            directives,
            fields,
        })
    }

    fn parse_interface_type_extension(&mut self) -> PResult<InterfaceTypeExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("interface")?;
        let name = self.parse_name()?;
        let interfaces = self.parse_implements_interfaces()?;
        let directives = self.parse_const_directives()?;
        let fields = self.parse_fields_definition()?;
        if interfaces.is_empty() && directives.is_empty() && fields.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(InterfaceTypeExtension {
            name,
            interfaces,
            directives,
            fields,
        })
    }

    fn parse_union_type_extension(&mut self) -> PResult<UnionTypeExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("union")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        let types = self.parse_union_member_types()?;
        if directives.is_empty() && types.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(UnionTypeExtension {
            name,
            directives,
            types,
        })
    }

    fn parse_enum_type_extension(&mut self) -> PResult<EnumTypeExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("enum")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        let values = self.parse_enum_values_definition()?;
        if directives.is_empty() && values.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(EnumTypeExtension {
            name,
            directives,
            values,
        })
    }

    fn parse_input_object_type_extension(&mut self) -> PResult<InputObjectTypeExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("input")?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        let fields = self.parse_input_fields_definition()?;
        if directives.is_empty() && fields.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(InputObjectTypeExtension {
            name,
            directives,
            fields,
        })
    }

    fn parse_directive_extension(&mut self) -> PResult<DirectiveExtension> {
        self.expect_keyword("extend")?;
        self.expect_keyword("directive")?;
        self.expect_token(TokenKind::At)?;
        let name = self.parse_name()?;
        let directives = self.parse_const_directives()?;
        if directives.is_empty() {
            return Err(unexpected(&self.lexer.token));
        }
        Ok(DirectiveExtension { name, directives })
    }

    fn parse_directive_definition(&mut self) -> PResult<DirectiveDefinition> {
        let description = self.parse_description()?;
        self.expect_keyword("directive")?;
        self.expect_token(TokenKind::At)?;
        let name = self.parse_name()?;
        let arguments = self.parse_argument_defs()?;
        let directives = self.parse_const_directives()?;
        let repeatable = self.expect_optional_keyword("repeatable")?;
        self.expect_keyword("on")?;
        let locations = self.parse_directive_locations()?;
        Ok(DirectiveDefinition {
            description,
            name,
            arguments,
            directives,
            repeatable,
            locations,
        })
    }

    fn parse_directive_locations(&mut self) -> PResult<Vec<Name>> {
        self.delimited_many(TokenKind::Pipe, Self::parse_directive_location)
    }

    fn parse_directive_location(&mut self) -> PResult<Name> {
        let name = self.parse_name()?;
        if DirectiveLocation::from_str(&name.value).is_some() {
            return Ok(name);
        }
        Err(unexpected_desc(TokenKind::Name, Some(&name.value)))
    }

    fn parse_schema_coordinate(&mut self) -> PResult<SchemaCoordinate> {
        let of_directive = self.expect_optional_token(TokenKind::At)?;
        let name = self.parse_name()?;
        let mut member_name = None;
        if !of_directive && self.expect_optional_token(TokenKind::Dot)? {
            member_name = Some(self.parse_name()?);
        }
        let mut argument_name = None;
        if (of_directive || member_name.is_some())
            && self.expect_optional_token(TokenKind::ParenL)?
        {
            argument_name = Some(self.parse_name()?);
            self.expect_token(TokenKind::Colon)?;
            self.expect_token(TokenKind::ParenR)?;
        }
        Ok(if of_directive {
            match argument_name {
                Some(argument_name) => {
                    SchemaCoordinate::DirectiveArgumentCoordinate(DirectiveArgumentCoordinate {
                        name,
                        argument_name,
                    })
                }
                None => SchemaCoordinate::DirectiveCoordinate(DirectiveCoordinate { name }),
            }
        } else if let Some(member_name) = member_name {
            match argument_name {
                Some(argument_name) => SchemaCoordinate::ArgumentCoordinate(ArgumentCoordinate {
                    name,
                    field_name: member_name,
                    argument_name,
                }),
                None => SchemaCoordinate::MemberCoordinate(MemberCoordinate { name, member_name }),
            }
        } else {
            SchemaCoordinate::TypeCoordinate(TypeCoordinate { name })
        })
    }

    // Token helpers.

    #[inline]
    fn peek(&self, kind: TokenKind) -> bool {
        self.lexer.token.kind == kind
    }

    fn expect_token(&mut self, kind: TokenKind) -> PResult<Token<'s>> {
        if self.lexer.token.kind == kind {
            return self.advance_lexer();
        }
        Err(GraphQLError::syntax(format!(
            "Expected {}, found {}.",
            token_kind_desc(kind),
            self.lexer.token.desc()
        )))
    }

    fn expect_optional_token(&mut self, kind: TokenKind) -> PResult<bool> {
        if self.lexer.token.kind == kind {
            self.advance_lexer()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect_keyword(&mut self, value: &str) -> PResult<()> {
        let token = &self.lexer.token;
        if token.kind == TokenKind::Name && token.value.as_deref() == Some(value) {
            self.advance_lexer()?;
            Ok(())
        } else {
            Err(GraphQLError::syntax(format!(
                "Expected \"{value}\", found {}.",
                token.desc()
            )))
        }
    }

    fn expect_optional_keyword(&mut self, value: &str) -> PResult<bool> {
        let token = &self.lexer.token;
        if token.kind == TokenKind::Name && token.value.as_deref() == Some(value) {
            self.advance_lexer()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn optional_many<T>(
        &mut self,
        open_kind: TokenKind,
        parse_fn: fn(&mut Self) -> PResult<T>,
        close_kind: TokenKind,
    ) -> PResult<Vec<T>> {
        if self.expect_optional_token(open_kind)? {
            let mut nodes = Vec::new();
            loop {
                nodes.push(parse_fn(self)?);
                if self.expect_optional_token(close_kind)? {
                    break;
                }
            }
            return Ok(nodes);
        }
        Ok(Vec::new())
    }

    fn many<T>(
        &mut self,
        open_kind: TokenKind,
        parse_fn: fn(&mut Self) -> PResult<T>,
        close_kind: TokenKind,
    ) -> PResult<Vec<T>> {
        self.expect_token(open_kind)?;
        let mut nodes = Vec::new();
        loop {
            nodes.push(parse_fn(self)?);
            if self.expect_optional_token(close_kind)? {
                break;
            }
        }
        Ok(nodes)
    }

    fn delimited_many<T>(
        &mut self,
        delimiter_kind: TokenKind,
        parse_fn: fn(&mut Self) -> PResult<T>,
    ) -> PResult<Vec<T>> {
        self.expect_optional_token(delimiter_kind)?;
        let mut nodes = Vec::new();
        loop {
            nodes.push(parse_fn(self)?);
            if !self.expect_optional_token(delimiter_kind)? {
                break;
            }
        }
        Ok(nodes)
    }

    /// graphql-js `advanceLexer`; returns the token that was current.
    fn advance_lexer(&mut self) -> PResult<Token<'s>> {
        let token = self.lexer.advance()?;
        if self.lexer.token.kind != TokenKind::Eof {
            self.token_counter += 1;
            if let Some(max_tokens) = self.options.max_tokens {
                if self.token_counter > max_tokens {
                    return Err(GraphQLError::syntax(format!(
                        "Document contains more than {max_tokens} tokens. Parsing aborted."
                    )));
                }
            }
        }
        Ok(token)
    }
}

fn token_value(token: Token<'_>) -> String {
    token.value.map(Cow::into_owned).unwrap_or_default()
}

/// graphql-js `parser.unexpected(token)`.
fn unexpected(token: &Token<'_>) -> Box<GraphQLError> {
    unexpected_desc(token.kind, token.value.as_deref())
}

fn unexpected_desc(kind: TokenKind, value: Option<&str>) -> Box<GraphQLError> {
    GraphQLError::syntax(format!("Unexpected {}.", token_desc(kind, value)))
}
