use std::collections::HashMap;
use typephp::ast::TypeExpr;
use typephp::codegen::{Codegen, PhpVersion};
use typephp::compile;
use typephp::error::Span;
use typephp::lexer::Lexer;
use typephp::parser::Parser;

// ============================================================================
// Codegen, PSR-12, PHPStan PHPDoc & Tree-shaking Tests (MC/DC) — 100+ cases
// ============================================================================

macro_rules! codegen_contains_tests {
    ($($name:ident => ($src:expr, $expected:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let php = compile($src).expect("should compile");
                assert!(
                    php.contains($expected),
                    "Expected output to contain {:?}\nActual:\n{}",
                    $expected,
                    php
                );
            }
        )*
    };
}

macro_rules! codegen_not_contains_tests {
    ($($name:ident => ($src:expr, $unexpected:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let php = compile($src).expect("should compile");
                assert!(
                    !php.contains($unexpected),
                    "Expected output NOT to contain {:?}\nActual:\n{}",
                    $unexpected,
                    php
                );
            }
        )*
    };
}

/// MC/DC D8: `use` tree-shaking & dedup: (!used_symbols.contains(short)) || (!seen_uses.insert(u))
#[test]
fn mcdc_use_tree_shaking_and_dedup() {
    let src = "<?php\nnamespace App;\nuse App\\Used;\nuse App\\Used;\nuse App\\Unused;\nclass Used {}\n$u = new Used();\n";
    let php = compile(src).unwrap();
    assert_eq!(php.matches("use App\\Used;").count(), 1);
    assert!(!php.contains("Unused"));
}

/// MC/DC D9: Type-only `import` elision: (!names.is_empty()) && names.iter().all(|n| aliases.contains_key(n))
#[test]
fn mcdc_type_only_import_elision() {
    let mut aliases = HashMap::new();
    aliases.insert("MyAlias".to_string(), TypeExpr::Named("int".into(), vec![], Span::new(1, 1)));

    let p1 = Parser::new(Lexer::new("import './side.tphp';").tokenize().unwrap()).parse_program().unwrap();
    assert!(Codegen::new().emit_program_with_aliases(&p1, &aliases).contains("require_once __DIR__ . '/side.php';"));

    let p2 = Parser::new(Lexer::new("import { MyAlias } from './types.tphp';").tokenize().unwrap()).parse_program().unwrap();
    assert!(!Codegen::new().emit_program_with_aliases(&p2, &aliases).contains("types.php"));

    let p3 = Parser::new(Lexer::new("import { MyAlias, helperFn } from './mixed.tphp';").tokenize().unwrap()).parse_program().unwrap();
    assert!(Codegen::new().emit_program_with_aliases(&p3, &aliases).contains("require_once __DIR__ . '/mixed.php';"));
}

#[test]
fn php_target_version_parsing_all_cases() {
    assert_eq!(PhpVersion::from_str("8.1"), Ok(PhpVersion::Php81));
    assert_eq!(PhpVersion::from_str("8.2"), Ok(PhpVersion::Php82));
    assert_eq!(PhpVersion::from_str("8.3"), Ok(PhpVersion::Php83));
    assert_eq!(PhpVersion::from_str("8.4"), Ok(PhpVersion::Php84));
    assert_eq!(PhpVersion::Php81.as_str(), "8.1");
    assert_eq!(PhpVersion::Php82.as_str(), "8.2");
    assert_eq!(PhpVersion::Php83.as_str(), "8.3");
    assert_eq!(PhpVersion::Php84.as_str(), "8.4");
    for bad in ["7.4", "8.0", "8.5", "9.0", "", "php84"] {
        assert!(PhpVersion::from_str(bad).is_err());
    }
}

#[test]
fn emit_bundle_handles_namespaces_and_internal_imports() {
    let p1 = Parser::new(Lexer::new("namespace Ns\\A;\nclass A {}\n").tokenize().unwrap()).parse_program().unwrap();
    let p2 = Parser::new(Lexer::new("import { A } from './A.tphp';\nimport './ext.php';\nuse Ns\\A\\A;\n$a = new A();\n").tokenize().unwrap()).parse_program().unwrap();
    let bundled = Codegen::new_with_target(PhpVersion::Php82).emit_bundle(&[&p1, &p2]);
    assert!(bundled.contains("// Target: PHP 8.2"));
    assert!(bundled.contains("namespace Ns\\A\n{"));
    assert!(bundled.contains("namespace\n{"));
    assert!(!bundled.contains("A.php"));
    assert!(bundled.contains("require_once __DIR__ . '/ext.php';"));
}

#[test]
fn emit_bundle_without_namespaces_stays_flat() {
    let p1 = Parser::new(Lexer::new("function f(): int { return 1; }").tokenize().unwrap()).parse_program().unwrap();
    let p2 = Parser::new(Lexer::new("import './f.tphp';\necho f();").tokenize().unwrap()).parse_program().unwrap();
    let bundled = Codegen::new().emit_bundle(&[&p1, &p2]);
    assert!(!bundled.contains("namespace"));
    assert!(bundled.contains("function f(): int"));
    assert!(bundled.contains("echo f();"));
}

// --- 55 Codegen positive checks ---
codegen_contains_tests! {
    cg_psr12_header => ("$x = 1;", "<?php\n\ndeclare(strict_types=1);"),
    cg_target_comment => ("$x = 1;", "// Target: PHP 8.4"),
    cg_allman_class => ("class C {}", "class C\n{\n}"),
    cg_allman_interface => ("interface I {}", "interface I\n{\n}"),
    cg_allman_enum => ("enum E { case A; }", "enum E\n{\n    case A;\n}"),
    cg_allman_fn => ("function f(): void {}", "function f(): void\n{\n}"),
    cg_allman_method => ("class C { public function m(): int { return 1; } }", "public function m(): int\n    {\n        return 1;\n    }"),
    cg_allman_ctor => ("class C { public function __construct() {} }", "public function __construct()\n    {\n    }"),
    cg_arrow_fn_space => ("$f = fn (int $x): int => $x;", "fn (int $x): int => $x"),
    cg_enum_string_case => ("enum S: string { case A = 'a'; }", "case A = 'a';"),
    cg_enum_int_case => ("enum S: int { case A = 1; }", "case A = 1;"),
    cg_class_extends => ("class A {} class B extends A {}", "class B extends A"),
    cg_class_implements => ("interface I {} class C implements I {}", "class C implements I"),
    cg_iface_extends => ("interface A {} interface B extends A {}", "interface B extends A"),
    cg_field_public => ("class C { public int $a = 1; }", "public int $a = 1;"),
    cg_field_private => ("class C { private string $b; }", "private string $b;"),
    cg_field_protected => ("class C { protected bool $c = true; }", "protected bool $c = true;"),
    cg_var_int => ("$x: int = 1;", "$x = 1;"),
    cg_var_float_whole => ("$x: float = 2.0;", "$x = 2.0;"),
    cg_var_bool_true => ("$x: bool = true;", "$x = true;"),
    cg_var_bool_false => ("$x: bool = false;", "$x = false;"),
    cg_var_null => ("$x: null = null;", "$x = null;"),
    cg_const_decl => ("const PI = 3.14;", "const PI = 3.14;"),
    cg_echo_stmt => ("echo 'hi';", "echo 'hi';"),
    cg_if_elseif_else => ("if (true) { echo 1; } elseif (false) { echo 2; } else { echo 3; }", "} elseif (false) {"),
    cg_while_stmt => ("while (false) { echo 1; }", "while (false) {"),
    cg_foreach_val => ("$a = [1]; foreach ($a as $v) { echo $v; }", "foreach ($a as $v) {"),
    cg_foreach_kv => ("$d: dict<string, int> = ['a' => 1]; foreach ($d as $k => $v) { echo $k; }", "foreach ($d as $k => $v) {"),
    cg_cast_int => ("$x = (int) '1';", "(int) '1'"),
    cg_cast_float => ("$x = (float) 1;", "(float) 1"),
    cg_cast_string => ("$x = (string) 1;", "(string) 1"),
    cg_cast_bool => ("$x = (bool) 1;", "(bool) 1"),
    cg_bin_add => ("$x = 1 + 2;", "1 + 2"),
    cg_bin_sub => ("$x = 1 - 2;", "1 - 2"),
    cg_bin_mul => ("$x = 1 * 2;", "1 * 2"),
    cg_bin_div => ("$x = 1 / 2;", "1 / 2"),
    cg_bin_mod => ("$x = 1 % 2;", "1 % 2"),
    cg_bin_concat => ("$x = 'a' . 'b';", "'a' . 'b'"),
    cg_bin_eq => ("$x = 1 === 1;", "1 === 1"),
    cg_bin_neq => ("$x = 1 !== 2;", "1 !== 2"),
    cg_bin_cmp => ("$x = 1 <= 2 && 3 >= 2 || 1 < 2 && 2 > 1;", "1 <= 2 && 3 >= 2 || 1 < 2 && 2 > 1"),
    cg_bin_null_coal => ("$x = null ?? 'a';", "null ?? 'a'"),
    cg_instanceof => ("class A {} $a = new A(); $b = $a instanceof A;", "$a instanceof A"),
    cg_static_access => ("enum E: string { case A = 'a'; } $x = E::A;", "E::A"),
    cg_phpdoc_template_fn => ("function id<T>(T $x): T { return $x; }", " * @template T"),
    cg_phpdoc_template_class => ("class B<T> { private T $v; public function __construct(T $v) { $this->v = $v; } }", " * @template T"),
    cg_phpdoc_list_var => ("$xs: int[] = [1];", "/** @var list<int> $xs */"),
    cg_phpdoc_dict_var => ("$d: dict<string, int> = ['a' => 1];", "/** @var array<string, int> $d */"),
    cg_phpdoc_shape_var => ("type S = { id: int, opt?: string }; $s: S = ['id' => 1];", "/** @var array{id: int, opt?: string} $s */"),
    cg_phpdoc_closure_param => ("function run((int) => string $cb): void {}", " * @param \\Closure(int): string $cb"),
    cg_phpdoc_return_list => ("function nums(): int[] { return [1]; }", " * @return list<int>"),
    cg_phpdoc_nullable_php_type => ("function f(?int $x): ?string { return null; }", "function f(?int $x): ?string"),
    cg_phpdoc_union_php_type => ("function f(int|string $x): int|string { return $x; }", "function f(int|string $x): int|string"),
    cg_any_to_mixed => ("function f(any $x): any { return $x; }", "function f(mixed $x): mixed"),
    cg_string_quote_escaping => ("$s = \"it's \\\\ok\";", "'it\\'s \\\\ok'"),
}

// --- 40 Codegen negative / erasure / tree-shaking checks ---
codegen_not_contains_tests! {
    cg_no_type_alias_id => ("type Id = int; $x: Id = 1;", "type Id"),
    cg_no_type_alias_shape => ("type S = { id: int };", "type S"),
    cg_no_type_alias_generic => ("type G<T> = { v: T };", "type G"),
    cg_no_generic_fn_tparams => ("function id<T>(T $x): T { return $x; }", "function id<T>"),
    cg_no_generic_call_targs => ("function id<T>(T $x): T { return $x; } $x = id<int>(1);", "id<int>"),
    cg_no_generic_new_targs => ("class B<T> { private T $v; public function __construct(T $v) { $this->v = $v; } } $b = new B<int>(1);", "new B<int>"),
    cg_no_generic_class_tparams => ("class B<T> {}", "class B<T>"),
    cg_no_unused_use_1 => ("namespace A; use A\\U1; class X {}", "U1"),
    cg_no_unused_use_2 => ("namespace A; use A\\U2; function f(): void {}", "U2"),
    cg_no_unused_use_3 => ("namespace A; use A\\U3; enum E { case K; }", "U3"),
    cg_no_unused_use_4 => ("namespace A; use A\\U4; interface I {}", "U4"),
    cg_no_unused_use_5 => ("namespace A; use A\\U5; $x = 1;", "U5"),
    cg_no_double_newline_eof => ("$x = 1;", "\n\n\n"),
    cg_no_fn_without_space => ("$f = fn (int $x): int => $x;", "fn("),
    cg_no_tphp_ext_in_req => ("require_once './foo.tphp';", "foo.tphp"),
    cg_no_tphp_ext_in_imp => ("import './bar.tphp';", "bar.tphp"),
    cg_no_any_keyword_in_param => ("function f(any $x): void {}", "(any $x)"),
    cg_no_any_keyword_in_ret => ("function f(): any { return 1; }", "): any"),
    cg_no_dict_keyword_in_param => ("function f(dict<string, int> $d): void {}", "(dict"),
    cg_no_shape_braces_in_php_sig => ("function f({ id: int } $s): void {}", "({ id"),
    cg_no_var_type_in_assign => ("$x: int = 1;", "$x: int"),
    cg_no_const_type_in_php => ("const X: int = 1;", "const X: int"),
    cg_no_same_line_class_brace => ("class A {}", "class A {"),
    cg_no_same_line_iface_brace => ("interface I {}", "interface I {"),
    cg_no_same_line_enum_brace => ("enum E { case A; }", "enum E {"),
    cg_no_same_line_fn_brace => ("function f(): void {}", "function f(): void {"),
    cg_no_same_line_method_brace => ("class A { public function m(): void {} }", "public function m(): void {"),
    cg_no_php_closing_tag => ("<?php $x = 1; ?>", "?>"),
    cg_no_double_require_1 => ("require_once './a.tphp'; require_once './a.tphp';", "a.php';\nrequire_once"),
    cg_no_double_require_2 => ("import './b.tphp'; import './b.tphp';", "b.php';\nrequire_once"),
    cg_no_double_use_1 => ("namespace N; use N\\C; use N\\C; class C {} $x = new C();", "use N\\C;\nuse N\\C;"),
    cg_no_unused_when_shadowed => ("namespace N; use N\\Missing; $x = 1;", "Missing"),
    cg_no_unused_type_alias_use => ("namespace N; use N\\Unused; type A = int;", "Unused"),
    cg_no_unused_in_dead_comment => ("namespace N; /* Foo */ use N\\Foo; $x = 1;", "use N\\Foo"),
    cg_no_unused_in_string => ("namespace N; use N\\Foo; $s = 'Foo';", "use N\\Foo"),
    cg_no_var_phpdoc_for_int => ("$x: int = 1;", "@var int"),
    cg_no_var_phpdoc_for_string => ("$x: string = 'a';", "@var string"),
    cg_no_var_phpdoc_for_bool => ("$x: bool = true;", "@var bool"),
    cg_no_var_phpdoc_for_float => ("$x: float = 1.0;", "@var float"),
    cg_no_fn_phpdoc_for_simple => ("function add(int $a, int $b): int { return $a + $b; }", "/**"),
}
