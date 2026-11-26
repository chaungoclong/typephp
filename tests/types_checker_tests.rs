use typephp::types::Type;
use typephp::{check, compile};

// ============================================================================
// Type System & TypeChecker Tests (MC/DC) — 100+ test cases
// ============================================================================

macro_rules! check_ok_tests {
    ($($name:ident => $src:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let res = check($src);
                assert!(res.is_ok(), "Expected check ok for {:?}, got {:?}", $src, res);
            }
        )*
    };
}

macro_rules! check_err_tests {
    ($($name:ident => $src:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let err = check($src).unwrap_err();
                assert_eq!(err.stage, "TypeError", "Expected TypeError for {:?}, got {:?}", $src, err);
            }
        )*
    };
}

// --- MC/DC D1..D7 (Unit & Integration) ---

#[test]
fn mcdc_d1_wildcard_and_never_assignability() {
    assert!(!Type::Int.is_assignable_to(&Type::String));
    assert!(Type::Int.is_assignable_to(&Type::Any));
    assert!(Type::Int.is_assignable_to(&Type::Mixed));
    assert!(Type::Any.is_assignable_to(&Type::String));
    assert!(Type::Never.is_assignable_to(&Type::String));
}

#[test]
fn mcdc_d2_shape_field_optionality_type_and_presence() {
    let req_target = Type::Shape(vec![("id".into(), false, Type::Int)]);
    let opt_target = Type::Shape(vec![("id".into(), true, Type::Int)]);

    assert!(Type::Shape(vec![("id".into(), false, Type::Int)]).is_assignable_to(&req_target));
    assert!(!Type::Shape(vec![("id".into(), true, Type::Int)]).is_assignable_to(&req_target));
    assert!(Type::Shape(vec![("id".into(), true, Type::Int)]).is_assignable_to(&opt_target));
    assert!(!Type::Shape(vec![("id".into(), false, Type::String)]).is_assignable_to(&req_target));
    assert!(!Type::Shape(vec![]).is_assignable_to(&req_target));
    assert!(Type::Shape(vec![]).is_assignable_to(&opt_target));
}

#[test]
fn mcdc_d3_function_type_assignability() {
    let target = Type::Function { params: vec![Type::Int], ret: Box::new(Type::Float) };
    assert!(Type::Function { params: vec![Type::Int], ret: Box::new(Type::Int) }.is_assignable_to(&target));
    assert!(!Type::Function { params: vec![Type::Int, Type::Int], ret: Box::new(Type::Int) }.is_assignable_to(&target));
    assert!(!Type::Function { params: vec![Type::String], ret: Box::new(Type::Int) }.is_assignable_to(&target));
    assert!(!Type::Function { params: vec![Type::Int], ret: Box::new(Type::String) }.is_assignable_to(&target));
}

#[test]
fn mcdc_d4_generic_class_assignability() {
    let target = Type::Class("Box".into(), vec![Type::Int]);
    assert!(Type::Class("Box".into(), vec![Type::Int]).is_assignable_to(&target));
    assert!(!Type::Class("Bag".into(), vec![Type::Int]).is_assignable_to(&target));
    assert!(!Type::Class("Box".into(), vec![Type::Int, Type::String]).is_assignable_to(&target));
    assert!(!Type::Class("Box".into(), vec![Type::String]).is_assignable_to(&target));
}

#[test]
fn mcdc_d5_union_source_and_target_assignability() {
    let u_int_str = Type::union(vec![Type::Int, Type::String]);
    let u_int_str_bool = Type::union(vec![Type::Int, Type::String, Type::Bool]);
    assert!(Type::Int.is_assignable_to(&u_int_str));
    assert!(!Type::Bool.is_assignable_to(&u_int_str));
    assert!(u_int_str.is_assignable_to(&u_int_str_bool));
    assert!(!u_int_str_bool.is_assignable_to(&u_int_str));
}

#[test]
fn mcdc_d6_array_dict_and_shape_to_dict_assignability() {
    assert!(Type::Array(Box::new(Type::Int)).is_assignable_to(&Type::Array(Box::new(Type::Float))));
    assert!(!Type::Array(Box::new(Type::String)).is_assignable_to(&Type::Array(Box::new(Type::Int))));
    assert!(Type::Array(Box::new(Type::Any)).is_assignable_to(&Type::Dict(Box::new(Type::String), Box::new(Type::Int))));
    assert!(Type::Dict(Box::new(Type::String), Box::new(Type::Int)).is_assignable_to(&Type::Dict(Box::new(Type::String), Box::new(Type::Float))));
    assert!(!Type::Dict(Box::new(Type::Int), Box::new(Type::Int)).is_assignable_to(&Type::Dict(Box::new(Type::String), Box::new(Type::Int))));
    let shape = Type::Shape(vec![("a".into(), false, Type::Int)]);
    assert!(shape.is_assignable_to(&Type::Dict(Box::new(Type::String), Box::new(Type::Int))));
    assert!(!shape.is_assignable_to(&Type::Dict(Box::new(Type::Int), Box::new(Type::Int))));
}

#[test]
fn mcdc_d7_type_substitution_and_remove_null() {
    let mut subst = std::collections::HashMap::new();
    subst.insert("T".into(), Type::Int);
    let ty = Type::union(vec![Type::Array(Box::new(Type::TypeParam("T".into()))), Type::Null]);
    let sub = ty.substitute(&subst);
    assert_eq!(sub, Type::union(vec![Type::Array(Box::new(Type::Int)), Type::Null]));
    assert_eq!(sub.remove_null(), Type::Array(Box::new(Type::Int)));
    assert_eq!(Type::Null.remove_null(), Type::Never);
}

// --- 50+ Valid TypeChecker Scenarios ---
check_ok_tests! {
    tc_ok_int_decl => "$x: int = 42;",
    tc_ok_float_decl => "$x: float = 3.14;",
    tc_ok_int_widens_to_float => "$x: float = 10;",
    tc_ok_string_decl => "$x: string = 'hello';",
    tc_ok_bool_decl => "$x: bool = false;",
    tc_ok_null_decl => "$x: null = null;",
    tc_ok_nullable_prefix => "$x: ?int = null; $y: ?int = 5;",
    tc_ok_nullable_postfix => "$x: string? = null; $y: string? = 'a';",
    tc_ok_union_decl => "$x: int|string = 1; $x = 'two';",
    tc_ok_any_escape => "$x: any = 1; $x = 's'; $y: int = $x;",
    tc_ok_mixed_decl => "$x: mixed = 1; $x = [1, 2];",
    tc_ok_const_inferred => "const A = 10; $x: int = A;",
    tc_ok_const_typed => "const B: string = 'ok'; $s: string = B;",
    tc_ok_infer_and_reassign => "$x = 1; $x = 2;",
    tc_ok_array_literal => "$a: int[] = [1, 2, 3];",
    tc_ok_empty_array_to_typed => "$a: string[] = [];",
    tc_ok_empty_array_to_dict => "$d: dict<string, int> = [];",
    tc_ok_dict_literal => "$d: dict<string, int> = ['a' => 1, 'b' => 2];",
    tc_ok_shape_exact => "type S = { id: int, name: string }; $u: S = ['id' => 1, 'name' => 'A'];",
    tc_ok_shape_optional_omitted => "type S = { id: int, email?: string }; $u: S = ['id' => 1];",
    tc_ok_shape_optional_present => "type S = { id: int, email?: string }; $u: S = ['id' => 1, 'email' => 'a@b.c'];",
    tc_ok_shape_index => "type S = { id: int, email?: string }; $u: S = ['id' => 1]; $id: int = $u['id']; $em: ?string = $u['email'];",
    tc_ok_arith_int => "$x: int = (1 + 2 - 3) * 4 / 2 % 2;",
    tc_ok_arith_float => "$x: float = 1 + 2.5;",
    tc_ok_unary_neg => "$x: int = -5; $y: float = -1.5;",
    tc_ok_unary_not => "$x: bool = !true;",
    tc_ok_concat_strings => "$s: string = 'a' . 'b';",
    tc_ok_concat_num => "$s: string = 'n=' . 1;",
    tc_ok_null_coal => "function f(?string $s): string { return $s ?? 'def'; }",
    tc_ok_casts => "$a: int = (int) '1'; $b: float = (float) 1; $c: string = (string) 1; $d: bool = (bool) 1;",
    tc_ok_fn_default_param => "function f(int $a, int $b = 1): int { return $a + $b; } $x: int = f(2);",
    tc_ok_fn_void_return => "function f(): void { return; }",
    tc_ok_arrow_fn => "$f = fn (int $x): int => $x * 2; $y: int = $f(3);",
    tc_ok_callback_param => "function apply(int $x, (int) => int $cb): int { return $cb($x); } $r: int = apply(2, fn (int $n): int => $n + 1);",
    tc_ok_generic_fn_explicit => "function id<T>(T $x): T { return $x; } $a: int = id<int>(1);",
    tc_ok_generic_fn_inferred => "function id<T>(T $x): T { return $x; } $a: string = id('hi');",
    tc_ok_generic_class => "class Box<T> { private T $v; public function __construct(T $v) { $this->v = $v; } public function get(): T { return $this->v; } } $b = new Box<int>(5); $x: int = $b->get();",
    tc_ok_generic_type_alias => "type Wrap<T> = { val: T }; $w: Wrap<bool> = ['val' => true]; $b: bool = $w['val'];",
    tc_ok_narrow_null_eq => "function f(?int $x): int { if ($x === null) { return 0; } return $x + 1; }",
    tc_ok_narrow_null_neq => "function f(?int $x): int { if ($x !== null) { return $x + 1; } return 0; }",
    tc_ok_narrow_is_int => "function f(int|string $x): int { if (is_int($x)) { return $x + 1; } return strlen($x); }",
    tc_ok_narrow_not_is_string => "function f(mixed $x): string { if (!is_string($x)) { return ''; } return trim($x); }",
    tc_ok_narrow_is_bool => "function f(bool|int $x): bool { if (is_bool($x)) { return !$x; } return $x > 0; }",
    tc_ok_narrow_is_float => "function f(float|string $x): float { if (is_float($x)) { return $x + 1.0; } return 0.0; }",
    tc_ok_narrow_instanceof => "interface I { public function ok(): bool; } class C implements I { public function ok(): bool { return true; } public function extra(): int { return 1; } } function f(I $i): int { if ($i instanceof C) { return $i->extra(); } return 0; }",
    tc_ok_interface_forward_ref => "interface A { public function b(): B; } interface B { public function n(): int; }",
    tc_ok_enum_access => "enum E: string { case X = 'x'; } $e: E = E::X;",
    tc_ok_foreach_array => "$xs: int[] = [1, 2]; foreach ($xs as $x) { $y: int = $x + 1; }",
    tc_ok_foreach_dict => "$d: dict<string, int> = ['a' => 1]; foreach ($d as $k => $v) { $s: string = $k; $n: int = $v; }",
    tc_ok_builtins_pdo => "$pdo = new PDO('sqlite::memory:'); $st: PDOStatement = $pdo->prepare('SELECT 1'); $st->execute([]); $rows: dict<int, mixed> = $st->fetchAll(PDO::FETCH_ASSOC);",
}

// --- 45+ Invalid TypeChecker Scenarios ---
check_err_tests! {
    tc_err_int_from_string => "$x: int = 'bad';",
    tc_err_string_from_int => "$x: string = 1;",
    tc_err_bool_from_int => "$x: bool = 1;",
    tc_err_null_from_int => "$x: null = 1;",
    tc_err_float_from_string => "$x: float = '1.5';",
    tc_err_reassign_mismatch => "$x = 1; $x = 'str';",
    tc_err_undef_var => "echo $nope;",
    tc_err_undef_ident => "echo NOPE;",
    tc_err_unknown_type => "$x: MissingType = 1;",
    tc_err_const_mismatch => "const C: int = 'no';",
    tc_err_arith_string_lhs => "$x = 'a' + 1;",
    tc_err_arith_bool_rhs => "$x = 1 + true;",
    tc_err_negate_string => "$x = -'a';",
    tc_err_concat_void => "function v(): void {} $x = v() . 'a';",
    tc_err_fn_too_few_args => "function f(int $a): int { return $a; } $x = f();",
    tc_err_fn_too_many_args => "function f(int $a): int { return $a; } $x = f(1, 2);",
    tc_err_fn_wrong_arg_type => "function f(int $a): int { return $a; } $x = f('a');",
    tc_err_fn_bad_default => "function f(int $a = 'x'): int { return $a; }",
    tc_err_fn_bad_return => "function f(): int { return 'x'; }",
    tc_err_fn_missing_return_val => "function f(): int { return; }",
    tc_err_arrow_bad_return => "$f = fn (int $x): int => 'no';",
    tc_err_call_non_callable => "$x = 5; $y = $x();",
    tc_err_cb_arity => "$f = fn (int $x): int => $x; $y = $f(1, 2);",
    tc_err_cb_arg_type => "$f = fn (int $x): int => $x; $y = $f('a');",
    tc_err_generic_fn_wrong_targs => "function id<T>(T $x): T { return $x; } $y = id<int, string>(1);",
    tc_err_generic_fn_mismatch => "function id<T>(T $x): T { return $x; } $y: int = id<string>('a');",
    tc_err_generic_class_wrong_targs => "class B<T> { private T $v; public function __construct(T $v) { $this->v = $v; } } $b = new B<int, int>(1);",
    tc_err_generic_alias_wrong_targs => "type P<A, B> = { a: A, b: B }; $x: P<int> = ['a' => 1];",
    tc_err_dict_wrong_targs => "$d: dict<string> = [];",
    tc_err_array_wrong_targs => "$a: array<int, string> = [];",
    tc_err_shape_missing_req => "type S = { id: int, name: string }; $u: S = ['id' => 1];",
    tc_err_shape_bad_field_type => "type S = { id: int }; $u: S = ['id' => 'no'];",
    tc_err_shape_unknown_key => "type S = { id: int }; $u: S = ['id' => 1]; $x = $u['missing'];",
    tc_err_array_non_int_index => "$a: int[] = [1]; $x = $a['0'];",
    tc_err_dict_bad_key_index => "$d: dict<string, int> = ['a' => 1]; $x = $d[0];",
    tc_err_index_non_indexable => "$x = 123; $y = $x[0];",
    tc_err_foreach_non_iterable => "$x = 123; foreach ($x as $i) {}",
    tc_err_unknown_interface => "class C implements NoSuchIface {}",
    tc_err_missing_iface_method => "interface I { public function f(): void; } class C implements I {}",
    tc_err_non_public_iface_method => "interface I { public function f(): void; } class C implements I { private function f(): void {} }",
    tc_err_iface_sig_mismatch => "interface I { public function f(): int; } class C implements I { public function f(): string { return ''; } }",
    tc_err_private_field_outside => "class C { private int $p = 1; } $c = new C(); $x = $c->p;",
    tc_err_private_method_outside => "class C { private function m(): int { return 1; } } $c = new C(); $x = $c->m();",
    tc_err_unknown_enum_case => "enum E: string { case A = 'a'; } $x = E::B;",
    tc_err_unknown_pdo_const => "$x = PDO::NO_SUCH_CONST;",
    tc_err_unknown_class_new => "$x = new NoSuchClass();",
    tc_err_unknown_instanceof => "$x = 1 instanceof NoSuchClass;",
    tc_err_void_if_cond => "function v(): void {} if (v()) {}",
}

#[test]
fn compile_generates_expected_php_for_generics_and_shapes() {
    let php = compile("type U = { id: int }; $u: U = ['id' => 1];").unwrap();
    assert!(php.contains("/** @var array{id: int} $u */"));
}
