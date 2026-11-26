use typephp::lexer::{Lexer, TokenKind};

// ============================================================================
// Lexer Tests (MC/DC) — 100+ test cases
// ============================================================================

macro_rules! lex_ok_tests {
    ($($name:ident => $src:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let res = Lexer::new($src).tokenize();
                assert!(res.is_ok(), "Expected lex ok for {:?}, got {:?}", $src, res);
            }
        )*
    };
}

macro_rules! lex_token_tests {
    ($($name:ident => ($src:expr, $pat:pat $(if $guard:expr)?)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let tokens = Lexer::new($src).tokenize().expect("lex ok");
                assert!(
                    matches!(&tokens[0].kind, $pat $(if $guard)?),
                    "Expected {:?} for {:?}, got {:?}",
                    stringify!($pat),
                    $src,
                    tokens[0].kind
                );
            }
        )*
    };
}

macro_rules! lex_err_tests {
    ($($name:ident => $src:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let err = Lexer::new($src).tokenize().unwrap_err();
                assert_eq!(err.stage, "LexError", "Expected LexError for {:?}", $src);
            }
        )*
    };
}

// --- 25 General Valid Lexer Inputs ---
lex_ok_tests! {
    lex_empty => "",
    lex_spaces_only => "     ",
    lex_newlines_crlf => "\r\n\r\n",
    lex_tabs_only => "\t\t\t",
    lex_php_open_close => "<?php $x = 1; ?>",
    lex_php_open_only => "<?php\n$x = 1;",
    lex_line_comment_slash => "// hello world\n$x = 1;",
    lex_line_comment_eof => "// comment at eof without newline",
    lex_hash_comment => "# hash comment\n$x = 1;",
    lex_hash_comment_eof => "# hash at eof",
    lex_block_comment_inline => "/* a * b */ $x = 1;",
    lex_block_comment_multiline => "/* line1\n * line2\n */ $x = 1;",
    lex_int_underscores => "1_000_000",
    lex_float_underscores => "1_234.5_67",
    lex_single_string_escapes => "'a\\'b\\\\c'",
    lex_double_string_escapes => "\"\\n\\r\\t\\\\\\\"\\'\\$x\"",
    lex_single_string_empty => "''",
    lex_double_string_empty => "\"\"",
    lex_multiline_string => "\"line1\nline2\"",
    lex_var_underscore_digits => "$_var_123",
    lex_ident_underscore_digits => "_Ident_99",
    lex_all_delimiters => "( ) { } [ ] , : ; . ? | & =",
    lex_all_cmp_and_logic => "=== !== == != < <= > >= && || ! ??",
    lex_all_arith_ops => "+ - * / %",
    lex_namespace_tokens => "namespace Foo\\Bar; use Foo\\Bar\\Baz;",
}

// --- 55 Exact TokenKind Mapping Tests (Literals, Keywords, Operators) ---
lex_token_tests! {
    tok_eof => ("", TokenKind::Eof),
    tok_int_zero => ("0", TokenKind::IntLit(0)),
    tok_int_val => ("42", TokenKind::IntLit(42)),
    tok_float_val => ("3.14", TokenKind::FloatLit(v) if (*v - 3.14).abs() < 1e-9),
    tok_string_single => ("'abc'", TokenKind::StringLit(s) if s == "abc"),
    tok_string_double => ("\"xyz\"", TokenKind::StringLit(s) if s == "xyz"),
    tok_bool_true => ("true", TokenKind::BoolLit(true)),
    tok_bool_false => ("false", TokenKind::BoolLit(false)),
    tok_null => ("null", TokenKind::NullLit),
    tok_var => ("$user", TokenKind::Var(s) if s == "user"),
    tok_ident => ("User", TokenKind::Ident(s) if s == "User"),
    tok_kw_const => ("const", TokenKind::Const),
    tok_kw_function => ("function", TokenKind::Function),
    tok_kw_fn => ("fn", TokenKind::Fn),
    tok_kw_return => ("return", TokenKind::Return),
    tok_kw_echo => ("echo", TokenKind::Echo),
    tok_kw_if => ("if", TokenKind::If),
    tok_kw_elseif => ("elseif", TokenKind::ElseIf),
    tok_kw_else => ("else", TokenKind::Else),
    tok_kw_while => ("while", TokenKind::While),
    tok_kw_foreach => ("foreach", TokenKind::ForEach),
    tok_kw_as => ("as", TokenKind::As),
    tok_kw_class => ("class", TokenKind::Class),
    tok_kw_interface => ("interface", TokenKind::Interface),
    tok_kw_implements => ("implements", TokenKind::Implements),
    tok_kw_extends => ("extends", TokenKind::Extends),
    tok_kw_enum => ("enum", TokenKind::Enum),
    tok_kw_case => ("case", TokenKind::Case),
    tok_kw_type => ("type", TokenKind::Type),
    tok_kw_public => ("public", TokenKind::Public),
    tok_kw_private => ("private", TokenKind::Private),
    tok_kw_protected => ("protected", TokenKind::Protected),
    tok_kw_new => ("new", TokenKind::New),
    tok_kw_instanceof => ("instanceof", TokenKind::InstanceOf),
    tok_kw_require_once => ("require_once", TokenKind::RequireOnce),
    tok_kw_import => ("import", TokenKind::Import),
    tok_kw_from => ("from", TokenKind::From),
    tok_kw_namespace => ("namespace", TokenKind::Namespace),
    tok_kw_use => ("use", TokenKind::Use),
    tok_sym_lparen => ("(", TokenKind::LParen),
    tok_sym_rparen => (")", TokenKind::RParen),
    tok_sym_lbrace => ("{", TokenKind::LBrace),
    tok_sym_rbrace => ("}", TokenKind::RBrace),
    tok_sym_lbracket => ("[", TokenKind::LBracket),
    tok_sym_rbracket => ("]", TokenKind::RBracket),
    tok_sym_comma => (",", TokenKind::Comma),
    tok_sym_colon => (":", TokenKind::Colon),
    tok_sym_double_colon => ("::", TokenKind::DoubleColon),
    tok_sym_semicolon => (";", TokenKind::Semicolon),
    tok_sym_dot => (".", TokenKind::Dot),
    tok_sym_backslash => ("\\", TokenKind::Backslash),
    tok_sym_arrow => ("->", TokenKind::Arrow),
    tok_sym_fat_arrow => ("=>", TokenKind::FatArrow),
    tok_sym_question => ("?", TokenKind::Question),
    tok_sym_null_coal => ("??", TokenKind::NullCoal),
}

// --- 20 Lexer Error Cases ---
lex_err_tests! {
    lex_err_unterminated_double_quote => "$x = \"abc;",
    lex_err_unterminated_single_quote => "$x = 'abc;",
    lex_err_unterminated_escape_double => "$x = \"abc\\",
    lex_err_unterminated_escape_single => "$x = 'abc\\",
    lex_err_unterminated_block_comment => "/* open comment",
    lex_err_unterminated_block_comment_star => "/* *",
    lex_err_dollar_eof => "$",
    lex_err_dollar_digit => "$1abc",
    lex_err_dollar_space => "$ abc",
    lex_err_dollar_sym => "$@abc",
    lex_err_dollar_dot => "$.abc",
    lex_err_at_char => "@",
    lex_err_tilde_char => "~",
    lex_err_caret_char => "^",
    lex_err_backtick_char => "`hello`",
    lex_err_non_ascii_euro => "€",
    lex_err_non_ascii_vn => "₫",
    lex_err_single_quote_newline_eof => "'hello\n",
    lex_err_double_quote_newline_eof => "\"hello\n",
    lex_err_block_comment_newline_unclosed => "/* a\nb",
}

// --- MC/DC & Span Accuracy Tests ---
#[test]
fn mcdc_lexer_float_dot_lookahead() {
    // `1.2` -> FloatLit (digit after `.`), `1 .` -> IntLit + Dot (non-digit after `.`)
    let t1 = Lexer::new("1.2").tokenize().unwrap();
    assert!(matches!(t1[0].kind, TokenKind::FloatLit(_)));
    let t2 = Lexer::new("1.a").tokenize().unwrap();
    assert!(matches!(t2[0].kind, TokenKind::IntLit(1)));
    assert!(matches!(t2[1].kind, TokenKind::Dot));
}

#[test]
fn lexer_span_tracks_multiline_positions() {
    let tokens = Lexer::new("<?php\n  $x = 1;\n    $y = 2;").tokenize().unwrap();
    let x_tok = tokens.iter().find(|t| matches!(&t.kind, TokenKind::Var(n) if n == "x")).unwrap();
    assert_eq!((x_tok.span.line, x_tok.span.col), (2, 3));
    let y_tok = tokens.iter().find(|t| matches!(&t.kind, TokenKind::Var(n) if n == "y")).unwrap();
    assert_eq!((y_tok.span.line, y_tok.span.col), (3, 5));
}
