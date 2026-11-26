use typephp::ast::{BinaryOp, Expr, Item, Stmt, TypeExpr, UnaryOp, Visibility};
use typephp::lexer::Lexer;
use typephp::parser::Parser;

// ============================================================================
// Parser Tests (MC/DC) — 100+ test cases
// ============================================================================

macro_rules! parse_ok_tests {
    ($($name:ident => $src:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let tokens = Lexer::new($src).tokenize().expect("lex ok");
                let res = Parser::new(tokens).parse_program();
                assert!(res.is_ok(), "Expected parse ok for {:?}, got {:?}", $src, res);
            }
        )*
    };
}

macro_rules! parse_err_tests {
    ($($name:ident => $src:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let tokens = Lexer::new($src).tokenize().expect("lex ok");
                let err = Parser::new(tokens).parse_program().unwrap_err();
                assert_eq!(err.stage, "ParseError", "Expected ParseError for {:?}", $src);
            }
        )*
    };
}

// --- 60 Valid Parser Cases ---
parse_ok_tests! {
    parse_empty_program => "",
    parse_namespace_simple => "namespace App;",
    parse_namespace_nested => "namespace App\\Http\\Controllers;",
    parse_use_simple => "use App\\User;",
    parse_use_leading_backslash => "use \\PDO;",
    parse_require_once => "require_once './a.php';",
    parse_import_default => "import './a.tphp';",
    parse_import_named_one => "import { Foo } from './a.tphp';",
    parse_import_named_many => "import { Foo, Bar, Baz } from './a.tphp';",
    parse_import_trailing_comma => "import { Foo, Bar, } from './a.tphp';",
    parse_type_alias_prim => "type Id = int;",
    parse_type_alias_generic_one => "type Box<T> = { v: T };",
    parse_type_alias_generic_two => "type Pair<A, B> = { a: A, b: B };",
    parse_type_alias_union_two => "type Val = int|string;",
    parse_type_alias_union_three => "type Val = int|string|null;",
    parse_type_alias_fn_zero => "type Thunk = () => void;",
    parse_type_alias_fn_two => "type Cb = (int, string) => bool;",
    parse_type_alias_prefix_nullable => "type MaybeStr = ?string;",
    parse_type_alias_postfix_nullable => "type MaybeInt = int?;",
    parse_type_alias_array_postfix => "type Nums = int[];",
    parse_type_alias_nested_array => "type Matrix = int[][];",
    parse_type_alias_shape_empty => "type EmptyShape = {};",
    parse_type_alias_shape_optional => "type U = { id: int, email?: string };",
    parse_type_alias_shape_semicolons => "type U = { id: int; name: string; };",
    parse_interface_empty => "interface Marker {}",
    parse_interface_extends_one => "interface A extends B {}",
    parse_interface_extends_many => "interface A extends B, C { public function f(int $x = 1): void; }",
    parse_interface_default_void => "interface I { public function ping(); }",
    parse_enum_unit => "enum Color { case Red; case Blue; }",
    parse_enum_string => "enum Role: string { case Admin = 'admin'; }",
    parse_enum_int => "enum Code: int { case Ok = 200; }",
    parse_class_empty => "class EmptyClass {}",
    parse_class_extends => "class Child extends ParentCls {}",
    parse_class_implements => "class Impl implements I1, I2 {}",
    parse_class_generic_full => "class C<T, U> extends Base implements I1, I2 {}",
    parse_class_fields_vis => "class F { public int $a = 1; private string $b; protected ?bool $c; }",
    parse_class_ctor => "class M { public function __construct(int $x) {} }",
    parse_class_methods => "class M { private function g<T>(T $v): T { return $v; } protected function h() {} }",
    parse_fn_void_implicit => "function noop() {}",
    parse_fn_void_explicit => "function noop(): void {}",
    parse_fn_generic_defaults => "function add<T>(int $a, int $b = 0): int { return $a + $b; }",
    parse_fn_untyped_param => "function legacy($x): mixed { return $x; }",
    parse_stmt_var_typed => "$x: int = 10;",
    parse_stmt_var_inferred => "$x = 10;",
    parse_stmt_const_typed => "const A: int = 1;",
    parse_stmt_const_inferred => "const A = 1;",
    parse_stmt_return_empty => "return;",
    parse_stmt_return_expr => "return 1 + 2;",
    parse_stmt_echo => "echo 'hello';",
    parse_stmt_if_only => "if (true) { echo 1; }",
    parse_stmt_if_else => "if (true) { echo 1; } else { echo 2; }",
    parse_stmt_if_elseif_else => "if ($a === 1) { echo 1; } elseif ($a === 2) { echo 2; } else { echo 3; }",
    parse_stmt_if_else_if => "if ($a) { echo 1; } else if ($b) { echo 2; }",
    parse_stmt_while => "while ($i < 10) { $i = $i + 1; }",
    parse_stmt_foreach_val => "foreach ($xs as $x) { echo $x; }",
    parse_stmt_foreach_kv => "foreach ($map as $k => $v) { echo $k; }",
    parse_expr_casts => "$a = (int) $x; $b = (float) $x; $c = (string) $x; $d = (bool) $x;",
    parse_expr_arrow_fn_no_ret => "$f = fn (int $x) => $x * 2;",
    parse_expr_arrow_fn_with_ret => "$f = fn (int $x): int => $x * 2;",
    parse_expr_postfix_chains => "$x = $a->b()->c[0]; $y = new Box<int>(1); $z = id<string>('s');",
}

// --- 35 Error Parser Cases ---
parse_err_tests! {
    parse_err_enum_bad_backing => "enum S: float { case A = 1.0; }",
    parse_err_enum_missing_case_kw => "enum S { Active = 'a'; }",
    parse_err_enum_missing_lbrace => "enum S case A;",
    parse_err_enum_missing_rbrace => "enum S { case A;",
    parse_err_if_missing_rparen => "if ($x === 1 { return; }",
    parse_err_if_missing_lparen => "if $x === 1) { return; }",
    parse_err_if_missing_lbrace => "if (true) return;",
    parse_err_while_missing_rparen => "while ($x < 1 { }",
    parse_err_while_missing_lparen => "while $x < 1) { }",
    parse_err_foreach_missing_as => "foreach ($xs $x) {}",
    parse_err_foreach_missing_var => "foreach ($xs as item) {}",
    parse_err_foreach_missing_val_after_fat_arrow => "foreach ($xs as $k => item) {}",
    parse_err_var_missing_type => "$x: = 1;",
    parse_err_var_missing_eq => "$x: int 1;",
    parse_err_const_missing_name => "const = 1;",
    parse_err_const_missing_eq => "const X: int 1;",
    parse_err_fn_missing_name => "function () {}",
    parse_err_fn_missing_rbrace => "function f() {",
    parse_err_param_missing_var => "function f(int x) {}",
    parse_err_param_missing_rparen => "function f(int $x {}",
    parse_err_class_missing_name => "class {}",
    parse_err_class_field_missing_var => "class A { public int x; }",
    parse_err_class_missing_rbrace => "class A {",
    parse_err_interface_missing_fn => "interface I { public int $x; }",
    parse_err_interface_missing_rbrace => "interface I {",
    parse_err_import_missing_from => "import { A } './a.tphp';",
    parse_err_import_bad_path => "import 123;",
    parse_err_require_bad_path => "require_once 123;",
    parse_err_assoc_non_string_key => "$x = [1 => 'a'];",
    parse_err_static_access_non_ident => "$x = $a::CONST;",
    parse_err_type_alias_missing_eq => "type A int;",
    parse_err_type_params_unclosed => "function f<T() {}",
    parse_err_fn_type_missing_fat_arrow => "type F = (int) -> int;",
    parse_err_unclosed_paren_expr => "$x = (1 + 2;",
    parse_err_unexpected_eof_expr => "$x = 1 +;",
}

// --- 7 Precedence & MC/DC Parser Structure Tests ---
#[test]
fn mcdc_enum_backing_type_accepts_string_int_rejects_others() {
    assert!(Parser::new(Lexer::new("enum A: string {}").tokenize().unwrap()).parse_program().is_ok());
    assert!(Parser::new(Lexer::new("enum B: int {}").tokenize().unwrap()).parse_program().is_ok());
    assert!(Parser::new(Lexer::new("enum C: bool {}").tokenize().unwrap()).parse_program().is_err());
}

#[test]
fn parser_precedence_mul_binds_tighter_than_add() {
    let prog = Parser::new(Lexer::new("$x = 1 + 2 * 3;").tokenize().unwrap()).parse_program().unwrap();
    let Item::Stmt(Stmt::Assign { value, .. }) = &prog.items[0] else { panic!("expected assign") };
    let Expr::Binary { op: BinaryOp::Add, right, .. } = value else { panic!("expected add") };
    assert!(matches!(right.as_ref(), Expr::Binary { op: BinaryOp::Mul, .. }));
}

#[test]
fn parser_precedence_and_binds_tighter_than_or() {
    let prog = Parser::new(Lexer::new("$x = true || false && false;").tokenize().unwrap()).parse_program().unwrap();
    let Item::Stmt(Stmt::Assign { value, .. }) = &prog.items[0] else { panic!("expected assign") };
    let Expr::Binary { op: BinaryOp::OrOr, right, .. } = value else { panic!("expected ||") };
    assert!(matches!(right.as_ref(), Expr::Binary { op: BinaryOp::AndAnd, .. }));
}

#[test]
fn parser_precedence_null_coal_lowest() {
    let prog = Parser::new(Lexer::new("$x = $a ?? $b || $c;").tokenize().unwrap()).parse_program().unwrap();
    let Item::Stmt(Stmt::Assign { value, .. }) = &prog.items[0] else { panic!("expected assign") };
    let Expr::Binary { op: BinaryOp::NullCoal, right, .. } = value else { panic!("expected ??") };
    assert!(matches!(right.as_ref(), Expr::Binary { op: BinaryOp::OrOr, .. }));
}

#[test]
fn parser_unary_neg_and_not_precedence() {
    let prog = Parser::new(Lexer::new("$x = -5; $y = !true;").tokenize().unwrap()).parse_program().unwrap();
    assert!(matches!(&prog.items[0], Item::Stmt(Stmt::Assign { value: Expr::Unary { op: UnaryOp::Neg, .. }, .. })));
    assert!(matches!(&prog.items[1], Item::Stmt(Stmt::Assign { value: Expr::Unary { op: UnaryOp::Not, .. }, .. })));
}

#[test]
fn parser_distinguishes_generic_call_from_less_than() {
    let prog = Parser::new(Lexer::new("$a = $x < 5; $b = id<int>(5);").tokenize().unwrap()).parse_program().unwrap();
    assert!(matches!(&prog.items[0], Item::Stmt(Stmt::Assign { value: Expr::Binary { op: BinaryOp::Lt, .. }, .. })));
    assert!(matches!(&prog.items[1], Item::Stmt(Stmt::Assign { value: Expr::Call { type_args, .. }, .. }) if type_args.len() == 1));
}

#[test]
fn parser_parses_default_public_visibility_on_class_members() {
    let prog = Parser::new(Lexer::new("class C { int $x; function m(): void {} }").tokenize().unwrap()).parse_program().unwrap();
    let Item::Class(cls) = &prog.items[0] else { panic!("expected class") };
    assert_eq!(cls.fields[0].visibility, Visibility::Public);
    assert_eq!(cls.methods[0].visibility, Visibility::Public);
    assert!(matches!(cls.fields[0].ty, TypeExpr::Named(ref n, _, _) if n == "int"));
}
