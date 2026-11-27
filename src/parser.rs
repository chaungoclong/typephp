use crate::ast::*;
use crate::error::{CompileError, Span};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, CompileError> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream always contains Eof")
        })
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &TokenKind, msg: &str) -> Result<Token, CompileError> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            Err(CompileError::parse(
                format!("{}, got {:?}", msg, self.peek().kind),
                self.peek().span,
            ))
        }
    }

    fn expect_ident(&mut self, msg: &str) -> Result<(String, Span), CompileError> {
        let tok = self.peek().clone();
        if let TokenKind::Ident(name) = tok.kind {
            self.advance();
            Ok((name, tok.span))
        } else {
            Err(CompileError::parse(
                format!("{}, got {:?}", msg, tok.kind),
                tok.span,
            ))
        }
    }

    fn expect_var(&mut self, msg: &str) -> Result<(String, Span), CompileError> {
        let tok = self.peek().clone();
        if let TokenKind::Var(name) = tok.kind {
            self.advance();
            Ok((name, tok.span))
        } else {
            Err(CompileError::parse(
                format!("{}, got {:?}", msg, tok.kind),
                tok.span,
            ))
        }
    }

    fn consume_semi(&mut self) {
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn parse_item(&mut self) -> Result<Item, CompileError> {
        match &self.peek().kind {
            TokenKind::Namespace => self.parse_namespace(),
            TokenKind::Use => self.parse_use(),
            TokenKind::RequireOnce => {
                let span = self.advance().span;
                let tok = self.advance();
                let path = match tok.kind {
                    TokenKind::StringLit(s) => s,
                    _ => {
                        return Err(CompileError::parse(
                            "Expected file path string after require_once",
                            tok.span,
                        ))
                    }
                };
                self.consume_semi();
                Ok(Item::Require(path, span))
            }
            TokenKind::Import => self.parse_import(),
            TokenKind::Type => Ok(Item::TypeAlias(self.parse_type_alias()?)),
            TokenKind::Interface => Ok(Item::Interface(self.parse_interface()?)),
            TokenKind::Enum => Ok(Item::Enum(self.parse_enum()?)),
            TokenKind::Class => Ok(Item::Class(self.parse_class()?)),
            TokenKind::Function => Ok(Item::Function(self.parse_fn()?)),
            _ => Ok(Item::Stmt(self.parse_stmt()?)),
        }
    }

    fn parse_qualified_name(&mut self) -> Result<String, CompileError> {
        let mut parts = Vec::new();
        if self.check(&TokenKind::Backslash) {
            self.advance();
            parts.push(String::new());
        }
        let (first, _) = self.expect_ident("Expected identifier in qualified name")?;
        parts.push(first);
        while self.check(&TokenKind::Backslash) {
            self.advance();
            let (seg, _) = self.expect_ident("Expected identifier after '\\'")?;
            parts.push(seg);
        }
        Ok(parts.join("\\"))
    }

    fn parse_namespace(&mut self) -> Result<Item, CompileError> {
        let span = self.advance().span;
        let ns = self.parse_qualified_name()?;
        self.consume_semi();
        Ok(Item::Namespace(ns, span))
    }

    fn parse_use(&mut self) -> Result<Item, CompileError> {
        let span = self.advance().span;
        let path = self.parse_qualified_name()?;
        self.consume_semi();
        Ok(Item::Use(path, span))
    }

    fn parse_import(&mut self) -> Result<Item, CompileError> {
        let span = self.advance().span; // import
        let mut names = Vec::new();
        if self.check(&TokenKind::LBrace) {
            self.advance();
            while !self.check(&TokenKind::RBrace) {
                let (name, _) = self.expect_ident("Expected imported symbol name")?;
                names.push(name);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&TokenKind::RBrace, "Expected '}' in import")?;
            self.expect(&TokenKind::From, "Expected 'from' in import")?;
        }
        let tok = self.advance();
        let path = match tok.kind {
            TokenKind::StringLit(s) => s,
            _ => {
                return Err(CompileError::parse(
                    "Expected module path string in import",
                    tok.span,
                ))
            }
        };
        self.consume_semi();
        Ok(Item::Import { names, path, span })
    }

    fn parse_type_params(&mut self) -> Result<Vec<String>, CompileError> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::Lt) {
            return Ok(params);
        }
        self.advance();
        while !self.check(&TokenKind::Gt) {
            let (name, _) = self.expect_ident("Expected type parameter name")?;
            params.push(name);
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::Gt, "Expected '>' after type parameters")?;
        Ok(params)
    }

    fn parse_type_alias(&mut self) -> Result<TypeAliasDecl, CompileError> {
        let span = self.advance().span; // type
        let (name, _) = self.expect_ident("Expected type alias name")?;
        let type_params = self.parse_type_params()?;
        self.expect(&TokenKind::Assign, "Expected '=' in type alias")?;
        let ty = self.parse_type_expr()?;
        self.consume_semi();
        Ok(TypeAliasDecl {
            name,
            type_params,
            ty,
            span,
        })
    }

    fn parse_visibility(&mut self) -> Visibility {
        match self.peek().kind {
            TokenKind::Public => {
                self.advance();
                Visibility::Public
            }
            TokenKind::Private => {
                self.advance();
                Visibility::Private
            }
            TokenKind::Protected => {
                self.advance();
                Visibility::Protected
            }
            _ => Visibility::Public,
        }
    }

    fn parse_interface(&mut self) -> Result<InterfaceDecl, CompileError> {
        let span = self.advance().span; // interface
        let (name, _) = self.expect_ident("Expected interface name")?;
        let mut extends = Vec::new();
        if self.check(&TokenKind::Extends) {
            self.advance();
            loop {
                extends.push(self.expect_ident("Expected parent interface name")?.0);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&TokenKind::LBrace, "Expected '{' in interface")?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let visibility = self.parse_visibility();
            let mspan = self
                .expect(&TokenKind::Function, "Expected 'function' in interface")?
                .span;
            let (mname, _) = self.expect_ident("Expected method name")?;
            let params = self.parse_params()?;
            let return_type = if self.check(&TokenKind::Colon) {
                self.advance();
                self.parse_type_expr()?
            } else {
                TypeExpr::Named("void".into(), vec![], mspan)
            };
            self.consume_semi();
            methods.push(MethodSig {
                visibility,
                name: mname,
                params,
                return_type,
                span: mspan,
            });
        }
        self.expect(&TokenKind::RBrace, "Expected '}' after interface")?;
        Ok(InterfaceDecl {
            name,
            extends,
            methods,
            span,
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDecl, CompileError> {
        let span = self.advance().span; // enum
        let (name, _) = self.expect_ident("Expected enum name")?;
        let backing_type = if self.check(&TokenKind::Colon) {
            self.advance();
            let (bname, bspan) = self.expect_ident("Expected backing type ('string' or 'int')")?;
            if bname != "string" && bname != "int" {
                return Err(CompileError::parse(
                    "Enum backing type must be 'string' or 'int'",
                    bspan,
                ));
            }
            Some(bname)
        } else {
            None
        };
        self.expect(&TokenKind::LBrace, "Expected '{' in enum")?;
        let mut cases = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            self.expect(&TokenKind::Case, "Expected 'case' in enum")?;
            let (cname, cspan) = self.expect_ident("Expected enum case name")?;
            let value = if self.check(&TokenKind::Assign) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.consume_semi();
            cases.push(EnumCase {
                name: cname,
                value,
                span: cspan,
            });
        }
        self.expect(&TokenKind::RBrace, "Expected '}' after enum")?;
        Ok(EnumDecl {
            name,
            backing_type,
            cases,
            span,
        })
    }

    fn parse_class(&mut self) -> Result<ClassDecl, CompileError> {
        let span = self.advance().span; // class
        let (name, _) = self.expect_ident("Expected class name")?;
        let type_params = self.parse_type_params()?;
        let extends = if self.check(&TokenKind::Extends) {
            self.advance();
            Some(self.expect_ident("Expected superclass name")?.0)
        } else {
            None
        };
        let mut implements = Vec::new();
        if self.check(&TokenKind::Implements) {
            self.advance();
            loop {
                implements.push(self.expect_ident("Expected interface name")?.0);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&TokenKind::LBrace, "Expected '{' in class")?;
        let mut fields = Vec::new();
        let mut constructor = None;
        let mut methods = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            let visibility = self.parse_visibility();
            if self.check(&TokenKind::Function) {
                let mspan = self.advance().span;
                let (mname, _) = self.expect_ident("Expected method name")?;
                if mname == "__construct" {
                    let params = self.parse_params()?;
                    let body = self.parse_block()?;
                    constructor = Some(ConstructorDecl {
                        visibility,
                        params,
                        body,
                        span: mspan,
                    });
                } else {
                    let mtype_params = self.parse_type_params()?;
                    let params = self.parse_params()?;
                    let return_type = if self.check(&TokenKind::Colon) {
                        self.advance();
                        self.parse_type_expr()?
                    } else {
                        TypeExpr::Named("void".into(), vec![], mspan)
                    };
                    let body = self.parse_block()?;
                    methods.push(MethodDecl {
                        visibility,
                        name: mname,
                        type_params: mtype_params,
                        params,
                        return_type,
                        body,
                        span: mspan,
                    });
                }
            } else {
                // PHP-style field: `private T $value;` or `public string $name = "a";`
                let ty = self.parse_type_expr()?;
                let (fname, fspan) = self.expect_var("Expected '$property' name in class field")?;
                let default = if self.check(&TokenKind::Assign) {
                    self.advance();
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.consume_semi();
                fields.push(ClassField {
                    visibility,
                    name: fname,
                    ty,
                    default,
                    span: fspan,
                });
            }
        }

        self.expect(&TokenKind::RBrace, "Expected '}' after class")?;
        Ok(ClassDecl {
            name,
            type_params,
            extends,
            implements,
            fields,
            constructor,
            methods,
            span,
        })
    }

    fn parse_fn(&mut self) -> Result<FnDecl, CompileError> {
        let span = self.advance().span; // function
        let (name, _) = self.expect_ident("Expected function name")?;
        let type_params = self.parse_type_params()?;
        let params = self.parse_params()?;
        let return_type = if self.check(&TokenKind::Colon) {
            self.advance();
            self.parse_type_expr()?
        } else {
            TypeExpr::Named("void".into(), vec![], span)
        };
        let body = self.parse_block()?;
        Ok(FnDecl {
            name,
            type_params,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, CompileError> {
        self.expect(&TokenKind::LParen, "Expected '('")?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) {
            // PHP style: `Type $name = default` (or untyped `$name`)
            let ty = if matches!(self.peek().kind, TokenKind::Var(_)) {
                TypeExpr::Named("mixed".into(), vec![], self.peek().span)
            } else {
                self.parse_type_expr()?
            };
            let (name, pspan) = self.expect_var("Expected '$param' name")?;
            let default = if self.check(&TokenKind::Assign) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            params.push(Param {
                name,
                ty,
                default,
                span: pspan,
            });
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "Expected ')'")?;
        Ok(params)
    }

    pub fn parse_type_expr(&mut self) -> Result<TypeExpr, CompileError> {
        let mut types = vec![self.parse_prefix_or_postfix_type()?];
        while self.check(&TokenKind::Pipe) {
            self.advance();
            types.push(self.parse_prefix_or_postfix_type()?);
        }
        if types.len() == 1 {
            Ok(types.into_iter().next().unwrap())
        } else {
            let span = types[0].span();
            Ok(TypeExpr::Union(types, span))
        }
    }

    fn parse_prefix_or_postfix_type(&mut self) -> Result<TypeExpr, CompileError> {
        if self.check(&TokenKind::Question) {
            let qspan = self.advance().span;
            let inner = self.parse_postfix_type()?;
            return Ok(TypeExpr::Optional(Box::new(inner), qspan));
        }
        self.parse_postfix_type()
    }

    fn parse_postfix_type(&mut self) -> Result<TypeExpr, CompileError> {
        let mut ty = self.parse_primary_type()?;
        loop {
            if self.check(&TokenKind::LBracket) {
                let bspan = self.advance().span;
                self.expect(&TokenKind::RBracket, "Expected ']' in array type")?;
                ty = TypeExpr::Array(Box::new(ty), bspan);
            } else if self.check(&TokenKind::Question) {
                let qspan = self.advance().span;
                ty = TypeExpr::Optional(Box::new(ty), qspan);
            } else {
                break;
            }
        }
        Ok(ty)
    }

    fn parse_primary_type(&mut self) -> Result<TypeExpr, CompileError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::NullLit => {
                self.advance();
                Ok(TypeExpr::Named("null".into(), vec![], tok.span))
            }
            TokenKind::Ident(name) => {
                self.advance();
                let mut args = Vec::new();
                if self.check(&TokenKind::Lt) {
                    self.advance();
                    while !self.check(&TokenKind::Gt) {
                        args.push(self.parse_type_expr()?);
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(&TokenKind::Gt, "Expected '>' after type arguments")?;
                }
                Ok(TypeExpr::Named(name, args, tok.span))
            }
            TokenKind::LBrace => {
                let span = self.advance().span;
                let mut fields = Vec::new();
                while !self.check(&TokenKind::RBrace) {
                    let (fname, fspan) = self.expect_ident("Expected shape field name")?;
                    let optional = if self.check(&TokenKind::Question) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    self.expect(&TokenKind::Colon, "Expected ':' in shape type")?;
                    let fty = self.parse_type_expr()?;
                    fields.push(ShapeField {
                        name: fname,
                        optional,
                        ty: fty,
                        span: fspan,
                    });
                    if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semicolon) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&TokenKind::RBrace, "Expected '}' in shape type")?;
                Ok(TypeExpr::Shape(fields, span))
            }
            TokenKind::LParen => {
                let span = self.advance().span;
                let mut params = Vec::new();
                while !self.check(&TokenKind::RParen) {
                    params.push(self.parse_type_expr()?);
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "Expected ')' in function type")?;
                self.expect(&TokenKind::FatArrow, "Expected '=>' in function type")?;
                let ret = self.parse_type_expr()?;
                Ok(TypeExpr::Function {
                    params,
                    ret: Box::new(ret),
                    span,
                })
            }
            _ => Err(CompileError::parse(
                format!("Expected type expression, got {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        self.expect(&TokenKind::LBrace, "Expected '{'")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace, "Expected '}'")?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, CompileError> {
        // Typed local variable: `$x: int = 10;`
        if let TokenKind::Var(name) = &self.peek().kind {
            if matches!(self.peek_at(1).map(|t| &t.kind), Some(TokenKind::Colon)) {
                let name = name.clone();
                let span = self.advance().span; // $var
                self.advance(); // ':'
                let ty = self.parse_type_expr()?;
                self.expect(&TokenKind::Assign, "Expected '=' after variable type")?;
                let init = self.parse_expr()?;
                self.consume_semi();
                return Ok(Stmt::VarDecl {
                    name,
                    ty: Some(ty),
                    init,
                    span,
                });
            }
        }

        match &self.peek().kind {
            TokenKind::Const => self.parse_const_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Echo => {
                let span = self.advance().span;
                let value = self.parse_expr()?;
                self.consume_semi();
                Ok(Stmt::Echo { value, span })
            }
            TokenKind::Global => {
                let span = self.advance().span;
                let mut vars = Vec::new();
                loop {
                    if let TokenKind::Var(name) = &self.peek().kind {
                        let vname = name.clone();
                        self.advance();
                        vars.push(vname);
                    } else {
                        return Err(CompileError::parse(
                            "Expected variable after 'global'",
                            self.peek().span,
                        ));
                    }
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.consume_semi();
                Ok(Stmt::Global { vars, span })
            }
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::ForEach => self.parse_foreach_stmt(),
            _ => {
                let expr = self.parse_expr()?;
                if self.check(&TokenKind::Assign) {
                    let span = self.advance().span;
                    let value = self.parse_expr()?;
                    self.consume_semi();
                    Ok(Stmt::Assign {
                        target: expr,
                        value,
                        span,
                    })
                } else {
                    let span = expr.span();
                    self.consume_semi();
                    Ok(Stmt::ExprStmt { expr, span })
                }
            }
        }
    }

    fn parse_const_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span; // const
        let (name, _) = self.expect_ident("Expected constant name")?;
        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&TokenKind::Assign, "Expected '=' in const declaration")?;
        let init = self.parse_expr()?;
        self.consume_semi();
        Ok(Stmt::ConstDecl {
            name,
            ty,
            init,
            span,
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span;
        let value = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::RBrace) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.consume_semi();
        Ok(Stmt::Return { value, span })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span;
        self.expect(&TokenKind::LParen, "Expected '(' after 'if'")?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::RParen, "Expected ')' after if condition")?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.check(&TokenKind::ElseIf) {
            Some(vec![self.parse_if_stmt()?])
        } else if self.check(&TokenKind::Else) {
            self.advance();
            if self.check(&TokenKind::If) {
                Some(vec![self.parse_if_stmt()?])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
            span,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span;
        self.expect(&TokenKind::LParen, "Expected '(' after 'while'")?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::RParen, "Expected ')' after while condition")?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body, span })
    }

    fn parse_foreach_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span; // foreach
        self.expect(&TokenKind::LParen, "Expected '(' after 'foreach'")?;
        let iterable = self.parse_expr()?;
        self.expect(&TokenKind::As, "Expected 'as' in foreach")?;
        let (first_var, _) = self.expect_var("Expected '$var' in foreach")?;
        let (key_var, val_var) = if self.check(&TokenKind::FatArrow) {
            self.advance();
            let (val, _) = self.expect_var("Expected '$value' after '=>' in foreach")?;
            (Some(first_var), val)
        } else {
            (None, first_var)
        };
        self.expect(&TokenKind::RParen, "Expected ')' after foreach header")?;
        let body = self.parse_block()?;
        Ok(Stmt::ForEach {
            iterable,
            key_var,
            val_var,
            body,
            span,
        })
    }

    pub fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        self.parse_null_coal()
    }

    fn parse_null_coal(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_or()?;
        while self.check(&TokenKind::NullCoal) {
            let span = self.advance().span;
            let right = self.parse_or()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::NullCoal,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::OrOr) {
            let span = self.advance().span;
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::OrOr,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::AndAnd) {
            let span = self.advance().span;
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::AndAnd,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_comparison()?;
        while matches!(
            self.peek().kind,
            TokenKind::EqEqEq | TokenKind::NotEqEq | TokenKind::EqEq | TokenKind::NotEq
        ) {
            let tok = self.advance();
            let op = match tok.kind {
                TokenKind::EqEqEq | TokenKind::EqEq => BinaryOp::EqEqEq,
                _ => BinaryOp::NotEqEq,
            };
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span: tok.span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_term()?;
        loop {
            if self.check(&TokenKind::InstanceOf) {
                let span = self.advance().span;
                let (class_name, _) = self.expect_ident("Expected class name after 'instanceof'")?;
                left = Expr::InstanceOf {
                    expr: Box::new(left),
                    class_name,
                    span,
                };
            } else if matches!(
                self.peek().kind,
                TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq
            ) {
                let tok = self.advance();
                let op = match tok.kind {
                    TokenKind::Lt => BinaryOp::Lt,
                    TokenKind::LtEq => BinaryOp::LtEq,
                    TokenKind::Gt => BinaryOp::Gt,
                    _ => BinaryOp::GtEq,
                };
                let right = self.parse_term()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    span: tok.span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_factor()?;
        while matches!(
            self.peek().kind,
            TokenKind::Plus | TokenKind::Minus | TokenKind::Dot
        ) {
            let tok = self.advance();
            let op = match tok.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => BinaryOp::Concat,
            };
            let right = self.parse_factor()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span: tok.span,
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_unary()?;
        while matches!(
            self.peek().kind,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent
        ) {
            let tok = self.advance();
            let op = match tok.kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => BinaryOp::Mod,
            };
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span: tok.span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        if matches!(self.peek().kind, TokenKind::Minus | TokenKind::Bang) {
            let tok = self.advance();
            let op = if matches!(tok.kind, TokenKind::Minus) {
                UnaryOp::Neg
            } else {
                UnaryOp::Not
            };
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
                span: tok.span,
            });
        }
        // PHP-style cast: (int)$x, (float)$x, (string)$x, (bool)$x
        if self.check(&TokenKind::LParen) {
            if let (Some(TokenKind::Ident(ty_name)), Some(TokenKind::RParen)) = (
                self.peek_at(1).map(|t| &t.kind),
                self.peek_at(2).map(|t| &t.kind),
            ) {
                if matches!(ty_name.as_str(), "int" | "float" | "string" | "bool") {
                    let target_ty = ty_name.clone();
                    let span = self.advance().span; // '('
                    self.advance(); // type
                    self.advance(); // ')'
                    let expr = self.parse_unary()?;
                    return Ok(Expr::Cast {
                        target_ty,
                        expr: Box::new(expr),
                        span,
                    });
                }
            }
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.check(&TokenKind::LParen) {
                let span = self.advance().span;
                let args = self.parse_args()?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    type_args: vec![],
                    args,
                    span,
                };
            } else if self.is_generic_call_ahead() {
                let span = self.peek().span;
                let type_args = self.parse_type_arg_list()?;
                self.expect(&TokenKind::LParen, "Expected '(' after generic type arguments")?;
                let args = self.parse_args()?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    type_args,
                    args,
                    span,
                };
            } else if self.check(&TokenKind::Arrow) {
                let span = self.advance().span;
                let (property, _) = self.expect_ident("Expected property or method name after '->'")?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    property,
                    span,
                };
            } else if self.check(&TokenKind::DoubleColon) {
                let span = self.advance().span;
                let (member, _) = self.expect_ident("Expected member name after '::'")?;
                let class_name = match expr {
                    Expr::Ident(name, _) => name,
                    _ => {
                        return Err(CompileError::parse(
                            "Expected class or enum name before '::'",
                            span,
                        ))
                    }
                };
                expr = Expr::StaticAccess {
                    class_name,
                    member,
                    span,
                };
            } else if self.check(&TokenKind::LBracket) {
                let span = self.advance().span;
                let index = self.parse_expr()?;
                self.expect(&TokenKind::RBracket, "Expected ']'")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn is_generic_call_ahead(&self) -> bool {
        if !self.check(&TokenKind::Lt) {
            return false;
        }
        let mut depth = 0usize;
        let mut i = self.pos;
        while i < self.tokens.len() {
            match self.tokens[i].kind {
                TokenKind::Lt => depth += 1,
                TokenKind::Gt => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(
                            self.tokens.get(i + 1).map(|t| &t.kind),
                            Some(TokenKind::LParen)
                        );
                    }
                }
                TokenKind::Semicolon | TokenKind::Eof | TokenKind::LBrace | TokenKind::RBrace => {
                    return false;
                }
                _ => {}
            }
            i += 1;
        }
        false
    }

    fn parse_type_arg_list(&mut self) -> Result<Vec<TypeExpr>, CompileError> {
        self.expect(&TokenKind::Lt, "Expected '<'")?;
        let mut args = Vec::new();
        while !self.check(&TokenKind::Gt) {
            args.push(self.parse_type_expr()?);
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::Gt, "Expected '>' after type arguments")?;
        Ok(args)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, CompileError> {
        let mut args = Vec::new();
        while !self.check(&TokenKind::RParen) {
            args.push(self.parse_expr()?);
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "Expected ')'")?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::IntLit(v) => {
                self.advance();
                Ok(Expr::Int(v, tok.span))
            }
            TokenKind::FloatLit(v) => {
                self.advance();
                Ok(Expr::Float(v, tok.span))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::String(s, tok.span))
            }
            TokenKind::BoolLit(b) => {
                self.advance();
                Ok(Expr::Bool(b, tok.span))
            }
            TokenKind::NullLit => {
                self.advance();
                Ok(Expr::Null(tok.span))
            }
            TokenKind::Var(name) => {
                self.advance();
                Ok(Expr::Var(name, tok.span))
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name, tok.span))
            }
            TokenKind::Fn => {
                self.advance();
                let params = self.parse_params()?;
                let return_type = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(self.parse_type_expr()?)
                } else {
                    None
                };
                self.expect(&TokenKind::FatArrow, "Expected '=>' in arrow function")?;
                let body = self.parse_expr()?;
                Ok(Expr::ArrowFn {
                    params,
                    return_type,
                    body: Box::new(body),
                    span: tok.span,
                })
            }
            TokenKind::New => {
                self.advance();
                let (class_name, _) = self.expect_ident("Expected class name after 'new'")?;
                let type_args = if self.check(&TokenKind::Lt) {
                    self.parse_type_arg_list()?
                } else {
                    vec![]
                };
                self.expect(&TokenKind::LParen, "Expected '(' in 'new' expression")?;
                let args = self.parse_args()?;
                Ok(Expr::New {
                    class_name,
                    type_args,
                    args,
                    span: tok.span,
                })
            }
            TokenKind::LBracket => {
                self.advance();
                if self.check(&TokenKind::RBracket) {
                    self.advance();
                    return Ok(Expr::ArrayLit(vec![], tok.span));
                }
                let first = self.parse_expr()?;
                if self.check(&TokenKind::FatArrow) {
                    // Associative array literal: ['key' => val, ...]
                    let key = match first {
                        Expr::String(s, _) => s,
                        _ => {
                            return Err(CompileError::parse(
                                "Expected string key in associative array literal",
                                first.span(),
                            ))
                        }
                    };
                    self.advance(); // =>
                    let val = self.parse_expr()?;
                    let mut entries = vec![(key, val)];
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                        let kexpr = self.parse_expr()?;
                        let k = match kexpr {
                            Expr::String(s, _) => s,
                            _ => {
                                return Err(CompileError::parse(
                                    "Expected string key in associative array literal",
                                    kexpr.span(),
                                ))
                            }
                        };
                        self.expect(&TokenKind::FatArrow, "Expected '=>' in associative array")?;
                        let v = self.parse_expr()?;
                        entries.push((k, v));
                    }
                    self.expect(&TokenKind::RBracket, "Expected ']'")?;
                    Ok(Expr::AssocArrayLit(entries, tok.span))
                } else {
                    let mut items = vec![first];
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                        items.push(self.parse_expr()?);
                    }
                    self.expect(&TokenKind::RBracket, "Expected ']'")?;
                    Ok(Expr::ArrayLit(items, tok.span))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "Expected ')'")?;
                Ok(expr)
            }
            _ => Err(CompileError::parse(
                format!("Unexpected token {:?} in expression", tok.kind),
                tok.span,
            )),
        }
    }
}
