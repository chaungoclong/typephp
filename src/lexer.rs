use crate::error::{CompileError, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    NullLit,

    // Variables ($name) & Identifiers (name)
    Var(String),
    Ident(String),

    // PHP / TypePHP Keywords
    Const,
    Function,
    Fn,
    Return,
    Echo,
    Global,
    If,
    ElseIf,
    Else,
    While,
    ForEach,
    As,
    Class,
    Interface,
    Implements,
    Extends,
    Enum,
    Case,
    Type,
    Public,
    Private,
    Protected,
    New,
    InstanceOf,
    RequireOnce,
    Import,
    From,
    Namespace,
    Use,

    // Symbols & Operators
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Comma,      // ,
    Colon,      // :
    DoubleColon,// ::
    Semicolon,  // ;
    Dot,        // .
    Backslash,  // \
    Arrow,      // ->
    FatArrow,   // =>
    Question,   // ?
    Pipe,       // |
    Amp,        // &
    Assign,     // =
    EqEqEq,     // ===
    NotEqEq,    // !==
    EqEq,       // ==
    NotEq,      // !=
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Bang,       // !
    AndAnd,     // &&
    OrOr,       // ||
    NullCoal,   // ??

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let span = Span::new(self.line, self.col);
            let Some(ch) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span,
                });
                break;
            };

            // Skip optional `<?php` opening tag
            if ch == '<' && self.starts_with("<?php") {
                for _ in 0..5 {
                    self.advance();
                }
                continue;
            }
            // Skip optional `?>` closing tag
            if ch == '?' && self.peek_next() == Some('>') {
                self.advance();
                self.advance();
                continue;
            }

            let kind = match ch {
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                '[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                ';' => {
                    self.advance();
                    TokenKind::Semicolon
                }
                '.' => {
                    self.advance();
                    TokenKind::Dot
                }
                '\\' => {
                    self.advance();
                    TokenKind::Backslash
                }
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
                }
                '%' => {
                    self.advance();
                    TokenKind::Percent
                }
                ':' => {
                    self.advance();
                    if self.peek() == Some(':') {
                        self.advance();
                        TokenKind::DoubleColon
                    } else {
                        TokenKind::Colon
                    }
                }
                '-' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '?' => {
                    self.advance();
                    if self.peek() == Some('?') {
                        self.advance();
                        TokenKind::NullCoal
                    } else {
                        TokenKind::Question
                    }
                }
                '|' => {
                    self.advance();
                    if self.peek() == Some('|') {
                        self.advance();
                        TokenKind::OrOr
                    } else {
                        TokenKind::Pipe
                    }
                }
                '&' => {
                    self.advance();
                    if self.peek() == Some('&') {
                        self.advance();
                        TokenKind::AndAnd
                    } else {
                        TokenKind::Amp
                    }
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::FatArrow
                    } else if self.peek() == Some('=') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::EqEqEq
                        } else {
                            TokenKind::EqEq
                        }
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::NotEqEq
                        } else {
                            TokenKind::NotEq
                        }
                    } else {
                        TokenKind::Bang
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LtEq
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                '$' => self.read_var(span)?,
                '"' | '\'' => self.read_string(ch, span)?,
                c if c.is_ascii_digit() => self.read_number(span)?,
                c if is_ident_start(c) => self.read_ident_or_keyword(),
                other => {
                    return Err(CompileError::lex(
                        format!("Unexpected character '{}'", other),
                        span,
                    ));
                }
            };

            tokens.push(Token { kind, span });
        }
        Ok(tokens)
    }

    fn starts_with(&self, s: &str) -> bool {
        let mut idx = self.pos;
        for expected in s.chars() {
            if self.chars.get(idx).copied() != Some(expected) {
                return false;
            }
            idx += 1;
        }
        true
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), CompileError> {
        loop {
            while let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            if (self.peek() == Some('/') && self.peek_next() == Some('/'))
                || self.peek() == Some('#')
            {
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                let start = Span::new(self.line, self.col);
                self.advance();
                self.advance();
                let mut closed = false;
                while let Some(c) = self.advance() {
                    if c == '*' && self.peek() == Some('/') {
                        self.advance();
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err(CompileError::lex("Unterminated block comment", start));
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    fn read_var(&mut self, span: Span) -> Result<TokenKind, CompileError> {
        self.advance(); // '$'
        let Some(first) = self.peek() else {
            return Err(CompileError::lex("Expected variable name after '$'", span));
        };
        if !is_ident_start(first) {
            return Err(CompileError::lex("Invalid variable name after '$'", span));
        }
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Ok(TokenKind::Var(name))
    }

    fn read_string(&mut self, quote: char, span: Span) -> Result<TokenKind, CompileError> {
        self.advance();
        let mut out = String::new();
        loop {
            let Some(ch) = self.advance() else {
                return Err(CompileError::lex("Unterminated string literal", span));
            };
            if ch == quote {
                break;
            } else if ch == '\\' {
                let Some(esc) = self.advance() else {
                    return Err(CompileError::lex("Unterminated escape sequence", span));
                };
                out.push(match esc {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    '\'' => '\'',
                    '$' => '$',
                    other => other,
                });
            } else {
                out.push(ch);
            }
        }
        Ok(TokenKind::StringLit(out))
    }

    fn read_number(&mut self, span: Span) -> Result<TokenKind, CompileError> {
        let mut raw = String::new();
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                raw.push(c);
                self.advance();
            } else if c == '.' && !is_float && self.peek_next().is_some_and(|n| n.is_ascii_digit())
            {
                is_float = true;
                raw.push(c);
                self.advance();
            } else if c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        if is_float {
            let val: f64 = raw
                .parse()
                .map_err(|_| CompileError::lex(format!("Invalid float literal '{}'", raw), span))?;
            Ok(TokenKind::FloatLit(val))
        } else {
            let val: i64 = raw
                .parse()
                .map_err(|_| CompileError::lex(format!("Invalid int literal '{}'", raw), span))?;
            Ok(TokenKind::IntLit(val))
        }
    }

    fn read_ident_or_keyword(&mut self) -> TokenKind {
        let mut ident = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match ident.as_str() {
            "true" => TokenKind::BoolLit(true),
            "false" => TokenKind::BoolLit(false),
            "null" => TokenKind::NullLit,
            "const" => TokenKind::Const,
            "function" => TokenKind::Function,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "echo" => TokenKind::Echo,
            "global" => TokenKind::Global,
            "if" => TokenKind::If,
            "elseif" => TokenKind::ElseIf,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "foreach" => TokenKind::ForEach,
            "as" => TokenKind::As,
            "class" => TokenKind::Class,
            "interface" => TokenKind::Interface,
            "implements" => TokenKind::Implements,
            "extends" => TokenKind::Extends,
            "enum" => TokenKind::Enum,
            "case" => TokenKind::Case,
            "type" => TokenKind::Type,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "protected" => TokenKind::Protected,
            "new" => TokenKind::New,
            "instanceof" => TokenKind::InstanceOf,
            "require_once" => TokenKind::RequireOnce,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "namespace" => TokenKind::Namespace,
            "use" => TokenKind::Use,
            _ => TokenKind::Ident(ident),
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
