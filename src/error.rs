use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub stage: &'static str,
    pub message: String,
    pub span: Span,
}

impl CompileError {
    pub fn lex(message: impl Into<String>, span: Span) -> Self {
        Self {
            stage: "LexError",
            message: message.into(),
            span,
        }
    }

    pub fn parse(message: impl Into<String>, span: Span) -> Self {
        Self {
            stage: "ParseError",
            message: message.into(),
            span,
        }
    }

    pub fn type_err(message: impl Into<String>, span: Span) -> Self {
        Self {
            stage: "TypeError",
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}:{}: {}",
            self.stage, self.span.line, self.span.col, self.message
        )
    }
}

impl std::error::Error for CompileError {}
