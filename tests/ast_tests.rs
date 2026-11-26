use typephp::ast::*;
use typephp::error::Span;
use typephp::lexer::Lexer;
use typephp::parser::Parser;

// ============================================================================
// AST Structure, Span & Invariant Tests — 100+ test cases
// ============================================================================

fn parse_one(src: &str) -> Program {
    let tokens = Lexer::new(src).tokenize().expect("lex ok");
    Parser::new(tokens).parse_program().expect("parse ok")
}

// --- 30 `Expr::span()` & `TypeExpr::span()` Tests ---
#[test]
fn ast_expr_span_int() { assert_eq!(Expr::Int(1, Span::new(2, 3)).span(), Span::new(2, 3)); }
#[test]
fn ast_expr_span_float() { assert_eq!(Expr::Float(1.0, Span::new(1, 5)).span(), Span::new(1, 5)); }
#[test]
fn ast_expr_span_string() { assert_eq!(Expr::String("a".into(), Span::new(3, 4)).span(), Span::new(3, 4)); }
#[test]
fn ast_expr_span_bool() { assert_eq!(Expr::Bool(true, Span::new(4, 1)).span(), Span::new(4, 1)); }
#[test]
fn ast_expr_span_null() { assert_eq!(Expr::Null(Span::new(5, 2)).span(), Span::new(5, 2)); }
#[test]
fn ast_expr_span_var() { assert_eq!(Expr::Var("x".into(), Span::new(6, 7)).span(), Span::new(6, 7)); }
#[test]
fn ast_expr_span_ident() { assert_eq!(Expr::Ident("F".into(), Span::new(7, 8)).span(), Span::new(7, 8)); }
#[test]
fn ast_expr_span_array_lit() { assert_eq!(Expr::ArrayLit(vec![], Span::new(8, 1)).span(), Span::new(8, 1)); }
#[test]
fn ast_expr_span_assoc_array_lit() { assert_eq!(Expr::AssocArrayLit(vec![], Span::new(9, 1)).span(), Span::new(9, 1)); }
#[test]
fn ast_expr_span_unary() {
    let e = Expr::Unary { op: UnaryOp::Neg, expr: Box::new(Expr::Int(1, Span::new(1, 2))), span: Span::new(1, 1) };
    assert_eq!(e.span(), Span::new(1, 1));
}
#[test]
fn ast_expr_span_cast() {
    let e = Expr::Cast { target_ty: "int".into(), expr: Box::new(Expr::Int(1, Span::new(1, 6))), span: Span::new(1, 1) };
    assert_eq!(e.span(), Span::new(1, 1));
}
#[test]
fn ast_expr_span_binary() {
    let e = Expr::Binary { left: Box::new(Expr::Int(1, Span::new(1, 1))), op: BinaryOp::Add, right: Box::new(Expr::Int(2, Span::new(1, 5))), span: Span::new(1, 3) };
    assert_eq!(e.span(), Span::new(1, 3));
}
#[test]
fn ast_expr_span_instanceof() {
    let e = Expr::InstanceOf { expr: Box::new(Expr::Var("x".into(), Span::new(1, 1))), class_name: "A".into(), span: Span::new(1, 4) };
    assert_eq!(e.span(), Span::new(1, 4));
}
#[test]
fn ast_expr_span_call() {
    let e = Expr::Call { callee: Box::new(Expr::Ident("f".into(), Span::new(1, 1))), type_args: vec![], args: vec![], span: Span::new(1, 2) };
    assert_eq!(e.span(), Span::new(1, 2));
}
#[test]
fn ast_expr_span_new() {
    let e = Expr::New { class_name: "A".into(), type_args: vec![], args: vec![], span: Span::new(2, 1) };
    assert_eq!(e.span(), Span::new(2, 1));
}
#[test]
fn ast_expr_span_member() {
    let e = Expr::Member { object: Box::new(Expr::Var("a".into(), Span::new(1, 1))), property: "p".into(), span: Span::new(1, 3) };
    assert_eq!(e.span(), Span::new(1, 3));
}
#[test]
fn ast_expr_span_static_access() {
    let e = Expr::StaticAccess { class_name: "E".into(), member: "A".into(), span: Span::new(1, 2) };
    assert_eq!(e.span(), Span::new(1, 2));
}
#[test]
fn ast_expr_span_index() {
    let e = Expr::Index { object: Box::new(Expr::Var("a".into(), Span::new(1, 1))), index: Box::new(Expr::Int(0, Span::new(1, 4))), span: Span::new(1, 3) };
    assert_eq!(e.span(), Span::new(1, 3));
}
#[test]
fn ast_expr_span_arrow_fn() {
    let e = Expr::ArrowFn { params: vec![], return_type: None, body: Box::new(Expr::Int(1, Span::new(1, 10))), span: Span::new(1, 1) };
    assert_eq!(e.span(), Span::new(1, 1));
}
#[test]
fn ast_type_span_named() { assert_eq!(TypeExpr::Named("int".into(), vec![], Span::new(1, 5)).span(), Span::new(1, 5)); }
#[test]
fn ast_type_span_array() { assert_eq!(TypeExpr::Array(Box::new(TypeExpr::Named("int".into(), vec![], Span::new(1, 1))), Span::new(1, 4)).span(), Span::new(1, 4)); }
#[test]
fn ast_type_span_union() { assert_eq!(TypeExpr::Union(vec![], Span::new(2, 3)).span(), Span::new(2, 3)); }
#[test]
fn ast_type_span_optional() { assert_eq!(TypeExpr::Optional(Box::new(TypeExpr::Named("int".into(), vec![], Span::new(1, 2))), Span::new(1, 1)).span(), Span::new(1, 1)); }
#[test]
fn ast_type_span_shape() { assert_eq!(TypeExpr::Shape(vec![], Span::new(3, 7)).span(), Span::new(3, 7)); }
#[test]
fn ast_type_span_function() { assert_eq!(TypeExpr::Function { params: vec![], ret: Box::new(TypeExpr::Named("void".into(), vec![], Span::new(1, 8))), span: Span::new(1, 1) }.span(), Span::new(1, 1)); }
#[test]
fn ast_program_equality_and_clone() {
    let p1 = parse_one("$x: int = 1;");
    let p2 = p1.clone();
    assert_eq!(p1, p2);
}
#[test]
fn ast_visibility_eq() {
    assert_ne!(Visibility::Public, Visibility::Private);
    assert_ne!(Visibility::Private, Visibility::Protected);
    assert_eq!(Visibility::Public, Visibility::Public);
}
#[test]
fn ast_unary_op_eq() { assert_ne!(UnaryOp::Neg, UnaryOp::Not); }
#[test]
fn ast_binary_op_eq() { assert_ne!(BinaryOp::Add, BinaryOp::Concat); assert_ne!(BinaryOp::EqEqEq, BinaryOp::NotEqEq); }
#[test]
fn ast_empty_program_items() { assert!(parse_one("").items.is_empty()); }

// --- 70+ Structural AST Node Verification Tests ---
macro_rules! ast_match_tests {
    ($($name:ident => ($src:expr, $pat:pat $(if $guard:expr)?)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let prog = parse_one($src);
                assert_eq!(prog.items.len(), 1, "Expected 1 top-level item for {:?}", $src);
                assert!(
                    matches!(&prog.items[0], $pat $(if $guard)?),
                    "AST mismatch for {:?}: got {:?}",
                    $src,
                    prog.items[0]
                );
            }
        )*
    };
}

ast_match_tests! {
    ast_item_namespace => ("namespace App\\Core;", Item::Namespace(ns, _) if ns == "App\\Core"),
    ast_item_use => ("use App\\Core\\User;", Item::Use(u, _) if u == "App\\Core\\User"),
    ast_item_require => ("require_once './a.php';", Item::Require(p, _) if p == "./a.php"),
    ast_item_import_empty_names => ("import './a.tphp';", Item::Import { names, path, .. } if names.is_empty() && path == "./a.tphp"),
    ast_item_import_named => ("import { A, B } from './a.tphp';", Item::Import { names, path, .. } if names == &["A", "B"] && path == "./a.tphp"),
    ast_item_type_alias => ("type Id = int;", Item::TypeAlias(ta) if ta.name == "Id" && ta.type_params.is_empty()),
    ast_item_type_alias_generic => ("type Box<T, U> = T;", Item::TypeAlias(ta) if ta.name == "Box" && ta.type_params == &["T", "U"]),
    ast_item_interface_basic => ("interface I { public function f(): void; }", Item::Interface(i) if i.name == "I" && i.methods.len() == 1),
    ast_item_interface_extends => ("interface I extends A, B {}", Item::Interface(i) if i.extends == &["A", "B"]),
    ast_item_enum_unit => ("enum E { case A; case B; }", Item::Enum(e) if e.name == "E" && e.backing_type.is_none() && e.cases.len() == 2),
    ast_item_enum_string => ("enum E: string { case A = 'a'; }", Item::Enum(e) if e.backing_type.as_deref() == Some("string") && e.cases[0].value.is_some()),
    ast_item_enum_int => ("enum E: int { case A = 1; }", Item::Enum(e) if e.backing_type.as_deref() == Some("int")),
    ast_item_class_basic => ("class C {}", Item::Class(c) if c.name == "C" && c.extends.is_none() && c.implements.is_empty()),
    ast_item_class_extends => ("class C extends Base {}", Item::Class(c) if c.extends.as_deref() == Some("Base")),
    ast_item_class_implements => ("class C implements I1, I2 {}", Item::Class(c) if c.implements == &["I1", "I2"]),
    ast_item_class_type_params => ("class C<T> {}", Item::Class(c) if c.type_params == &["T"]),
    ast_item_class_fields => ("class C { public int $a; private string $b = 'x'; }", Item::Class(c) if c.fields.len() == 2 && c.fields[1].visibility == Visibility::Private),
    ast_item_class_ctor => ("class C { public function __construct(int $x) {} }", Item::Class(c) if c.constructor.is_some()),
    ast_item_class_methods => ("class C { protected function m(): int { return 1; } }", Item::Class(c) if c.methods.len() == 1 && c.methods[0].visibility == Visibility::Protected),
    ast_item_fn_basic => ("function f(int $a): int { return $a; }", Item::Function(f) if f.name == "f" && f.params.len() == 1),
    ast_item_fn_generic => ("function id<T>(T $x): T { return $x; }", Item::Function(f) if f.type_params == &["T"]),
    ast_item_fn_default_param => ("function f(int $a = 5): void {}", Item::Function(f) if f.params[0].default.is_some()),
    ast_stmt_var_decl => ("$x: int = 1;", Item::Stmt(Stmt::VarDecl { name, ty: Some(_), .. }) if name == "x"),
    ast_stmt_const_typed => ("const C: int = 1;", Item::Stmt(Stmt::ConstDecl { name, ty: Some(_), .. }) if name == "C"),
    ast_stmt_const_inferred => ("const C = 1;", Item::Stmt(Stmt::ConstDecl { name, ty: None, .. }) if name == "C"),
    ast_stmt_assign_var => ("$x = 1;", Item::Stmt(Stmt::Assign { target: Expr::Var(n, _), .. }) if n == "x"),
    ast_stmt_assign_member => ("$this->x = 1;", Item::Stmt(Stmt::Assign { target: Expr::Member { property, .. }, .. }) if property == "x"),
    ast_stmt_assign_index => ("$a[0] = 1;", Item::Stmt(Stmt::Assign { target: Expr::Index { .. }, .. })),
    ast_stmt_return_none => ("return;", Item::Stmt(Stmt::Return { value: None, .. })),
    ast_stmt_return_some => ("return 1;", Item::Stmt(Stmt::Return { value: Some(Expr::Int(1, _)), .. })),
    ast_stmt_echo => ("echo 'hi';", Item::Stmt(Stmt::Echo { value: Expr::String(s, _), .. }) if s == "hi"),
    ast_stmt_if_no_else => ("if (true) {}", Item::Stmt(Stmt::If { else_branch: None, .. })),
    ast_stmt_if_with_else => ("if (true) {} else {}", Item::Stmt(Stmt::If { else_branch: Some(_), .. })),
    ast_stmt_if_with_elseif => ("if (true) {} elseif (false) {}", Item::Stmt(Stmt::If { else_branch: Some(eb), .. }) if matches!(eb.as_slice(), [Stmt::If { .. }])),
    ast_stmt_while => ("while (true) {}", Item::Stmt(Stmt::While { .. })),
    ast_stmt_foreach_val => ("foreach ($xs as $x) {}", Item::Stmt(Stmt::ForEach { key_var: None, val_var, .. }) if val_var == "x"),
    ast_stmt_foreach_kv => ("foreach ($xs as $k => $v) {}", Item::Stmt(Stmt::ForEach { key_var: Some(k), val_var, .. }) if k == "k" && val_var == "v"),
    ast_stmt_expr => ("foo();", Item::Stmt(Stmt::ExprStmt { expr: Expr::Call { .. }, .. })),
    ast_expr_int => ("1;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Int(1, _), .. })),
    ast_expr_float => ("1.5;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Float(_, _), .. })),
    ast_expr_string => ("'s';", Item::Stmt(Stmt::ExprStmt { expr: Expr::String(s, _), .. }) if s == "s"),
    ast_expr_bool_true => ("true;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Bool(true, _), .. })),
    ast_expr_bool_false => ("false;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Bool(false, _), .. })),
    ast_expr_null => ("null;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Null(_), .. })),
    ast_expr_var => ("$v;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Var(v, _), .. }) if v == "v"),
    ast_expr_ident => ("C;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Ident(i, _), .. }) if i == "C"),
    ast_expr_empty_array => ("[];", Item::Stmt(Stmt::ExprStmt { expr: Expr::ArrayLit(a, _), .. }) if a.is_empty()),
    ast_expr_list_array => ("[1, 2];", Item::Stmt(Stmt::ExprStmt { expr: Expr::ArrayLit(a, _), .. }) if a.len() == 2),
    ast_expr_assoc_array => ("['a' => 1];", Item::Stmt(Stmt::ExprStmt { expr: Expr::AssocArrayLit(e, _), .. }) if e.len() == 1 && e[0].0 == "a"),
    ast_expr_unary_neg => ("-1;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Unary { op: UnaryOp::Neg, .. }, .. })),
    ast_expr_unary_not => ("!true;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Unary { op: UnaryOp::Not, .. }, .. })),
    ast_expr_cast_int => ("(int) $x;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Cast { target_ty, .. }, .. }) if target_ty == "int"),
    ast_expr_cast_string => ("(string) $x;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Cast { target_ty, .. }, .. }) if target_ty == "string"),
    ast_expr_bin_add => ("1 + 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Add, .. }, .. })),
    ast_expr_bin_sub => ("1 - 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Sub, .. }, .. })),
    ast_expr_bin_mul => ("1 * 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Mul, .. }, .. })),
    ast_expr_bin_div => ("1 / 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Div, .. }, .. })),
    ast_expr_bin_mod => ("1 % 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Mod, .. }, .. })),
    ast_expr_bin_concat => ("'a' . 'b';", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Concat, .. }, .. })),
    ast_expr_bin_eq => ("1 === 1;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::EqEqEq, .. }, .. })),
    ast_expr_bin_neq => ("1 !== 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::NotEqEq, .. }, .. })),
    ast_expr_bin_lt => ("1 < 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Lt, .. }, .. })),
    ast_expr_bin_lte => ("1 <= 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::LtEq, .. }, .. })),
    ast_expr_bin_gt => ("1 > 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::Gt, .. }, .. })),
    ast_expr_bin_gte => ("1 >= 2;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::GtEq, .. }, .. })),
    ast_expr_bin_and => ("true && false;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::AndAnd, .. }, .. })),
    ast_expr_bin_or => ("true || false;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::OrOr, .. }, .. })),
    ast_expr_bin_null_coal => ("null ?? 1;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Binary { op: BinaryOp::NullCoal, .. }, .. })),
    ast_expr_instanceof => ("$x instanceof Foo;", Item::Stmt(Stmt::ExprStmt { expr: Expr::InstanceOf { class_name, .. }, .. }) if class_name == "Foo"),
    ast_expr_new => ("new Box<int>(1);", Item::Stmt(Stmt::ExprStmt { expr: Expr::New { class_name, type_args, args, .. }, .. }) if class_name == "Box" && type_args.len() == 1 && args.len() == 1),
    ast_expr_member => ("$a->b;", Item::Stmt(Stmt::ExprStmt { expr: Expr::Member { property, .. }, .. }) if property == "b"),
    ast_expr_static => ("E::A;", Item::Stmt(Stmt::ExprStmt { expr: Expr::StaticAccess { class_name, member, .. }, .. }) if class_name == "E" && member == "A"),
    ast_expr_index => ("$a[0];", Item::Stmt(Stmt::ExprStmt { expr: Expr::Index { .. }, .. })),
    ast_expr_arrow => ("fn (int $x): int => $x;", Item::Stmt(Stmt::ExprStmt { expr: Expr::ArrowFn { params, return_type: Some(_), .. }, .. }) if params.len() == 1),
}
