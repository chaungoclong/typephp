use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use typephp::codegen::{EmitMode, PhpVersion};
use typephp::{check_file, compile, compile_with_options, CompileOptions};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("typephp_e2e_{}_{}_{}", prefix, nanos, seq));
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ============================================================================
// End-to-End Compilation, Multi-File Graphs, PHPStan L9 & PHP Execution (100+)
// ============================================================================

#[test]
fn e2e_preserve_and_bundle_with_phpstan_and_php_exec() {
    let dir = temp_dir("full");
    fs::write(
        dir.join("Types.tphp"),
        "<?php\nnamespace Demo\\Types;\ntype UserShape = { id: int, name: string };\n",
    ).unwrap();
    fs::write(
        dir.join("Helper.tphp"),
        "<?php\nnamespace Demo\\Core;\nclass Greeter {\n    public function greet(string $name): string {\n        return 'Hello, ' . $name;\n    }\n}\n",
    ).unwrap();
    let entry = dir.join("main.tphp");
    fs::write(
        &entry,
        "<?php\nimport { UserShape } from './Types.tphp';\nimport { Greeter } from './Helper.tphp';\nuse Demo\\Core\\Greeter;\n$user: UserShape = ['id' => 1, 'name' => 'Long'];\n$g = new Greeter();\n$msg: string = $g->greet($user['name']);\necho $msg;\n",
    ).unwrap();

    // Preserve mode
    let out_dir = dir.join("dist");
    let preserve_opts = CompileOptions {
        target: PhpVersion::Php83,
        mode: EmitMode::Preserve,
        output: Some(out_dir.clone()),
        minify: false,
    };
    assert_eq!(compile_with_options(&entry, &preserve_opts).unwrap().len(), 3);
    let main_php = fs::read_to_string(out_dir.join("main.php")).unwrap();
    assert!(!main_php.contains("Types.php"));
    assert!(main_php.contains("require_once __DIR__ . '/Helper.php';"));

    // Bundle + Minify mode
    let bundle_out = dir.join("bundle.min.php");
    let bundle_opts = CompileOptions {
        target: PhpVersion::Php82,
        mode: EmitMode::Bundle,
        output: Some(bundle_out.clone()),
        minify: true,
    };
    assert_eq!(compile_with_options(&entry, &bundle_opts).unwrap().len(), 1);
    let run = Command::new("php").arg(&bundle_out).output().unwrap();
    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout), "Hello, Long");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn e2e_phpstan_level9_rejects_unsafe_compiled_code() {
    let dir = temp_dir("phpstan_fail");
    let entry = dir.join("unsafe.tphp");
    // TypePHP allows `any` -> `string`, but PHPStan L9 rejects `mixed` passed to `strlen(string)`
    fs::write(&entry, "<?php\n$x: any = 123;\n$n: int = strlen($x);\n").unwrap();
    let opts = CompileOptions {
        target: PhpVersion::Php84,
        mode: EmitMode::Bundle,
        output: Some(dir.join("unsafe.php")),
        minify: false,
    };
    let err = compile_with_options(&entry, &opts).unwrap_err();
    assert!(err.message.contains("PHPStan level 9 failed"));
    let _ = fs::remove_dir_all(&dir);
}

// --- 50 PHP Execution End-to-End Tests (compile -> php CLI -> assert stdout) ---
macro_rules! php_exec_tests {
    ($($name:ident => ($src:expr, $expected_stdout:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let php = compile($src).expect("should compile");
                let dir = temp_dir(stringify!($name));
                let file = dir.join("run.php");
                fs::write(&file, php).unwrap();
                let out = Command::new("php").arg(&file).output().unwrap();
                let _ = fs::remove_dir_all(&dir);
                assert!(out.status.success(), "php failed: {}", String::from_utf8_lossy(&out.stderr));
                assert_eq!(String::from_utf8_lossy(&out.stdout), $expected_stdout);
            }
        )*
    };
}

php_exec_tests! {
    exec_hello_world => ("echo 'Hello TypePHP';", "Hello TypePHP"),
    exec_int_arith => ("$a: int = 10; $b: int = 3; echo (string) ($a + $b * 2);", "16"),
    exec_float_arith => ("$a: float = 2.5; echo (string) ($a * 2.0);", "5"),
    exec_bool_ternary_via_if => ("$ok: bool = true; if ($ok) { echo 'yes'; } else { echo 'no'; }", "yes"),
    exec_null_coalesce => ("$x: ?string = null; echo $x ?? 'fallback';", "fallback"),
    exec_concat => ("echo 'A' . '-' . 'B';", "A-B"),
    exec_fn_default => ("function g(string $s = 'world'): string { return 'hi ' . $s; } echo g();", "hi world"),
    exec_fn_override_default => ("function g(string $s = 'world'): string { return 'hi ' . $s; } echo g('rust');", "hi rust"),
    exec_generic_fn_int => ("function id<T>(T $x): T { return $x; } echo (string) id<int>(77);", "77"),
    exec_generic_fn_str => ("function id<T>(T $x): T { return $x; } echo id('abc');", "abc"),
    exec_generic_class => ("class Box<T> { private T $v; public function __construct(T $v) { $this->v = $v; } public function get(): T { return $this->v; } } $b = new Box<string>('ok'); echo $b->get();", "ok"),
    exec_shape_access => ("type U = { id: int, name: string }; $u: U = ['id' => 9, 'name' => 'Ada']; echo $u['name'];", "Ada"),
    exec_array_foreach => ("$xs: int[] = [1, 2, 3, 4]; $s: int = 0; foreach ($xs as $x) { $s = $s + $x; } echo (string) $s;", "10"),
    exec_dict_foreach => ("$d: dict<string, int> = ['a' => 2, 'b' => 5]; $s: int = 0; foreach ($d as $k => $v) { $s = $s + $v; } echo (string) $s;", "7"),
    exec_while_loop => ("$i: int = 0; while ($i < 3) { $i = $i + 1; } echo (string) $i;", "3"),
    exec_if_elseif_else => ("$n: int = 2; if ($n === 1) { echo 'one'; } elseif ($n === 2) { echo 'two'; } else { echo 'other'; }", "two"),
    exec_arrow_fn => ("$mul = fn (int $x): int => $x * 3; echo (string) $mul(4);", "12"),
    exec_callback_fn => ("function run(int $v, (int) => int $f): int { return $f($v); } echo (string) run(5, fn (int $n): int => $n + 10);", "15"),
    exec_enum_string => ("enum Role: string { case Admin = 'admin'; } echo Role::Admin->value;", "admin"),
    exec_enum_int => ("enum Code: int { case Ok = 200; } echo (string) Code::Ok->value;", "200"),
    exec_interface_impl => ("interface I { public function ping(): string; } class C implements I { public function ping(): string { return 'pong'; } } $c = new C(); echo $c->ping();", "pong"),
    exec_narrow_is_int => ("function f(int|string $x): string { if (is_int($x)) { return 'I:' . (string) $x; } return 'S:' . $x; } echo f(5) . ',' . f('a');", "I:5,S:a"),
    exec_narrow_not_is_str => ("function f(mixed $v): string { if (!is_string($v)) { return 'no'; } return trim($v); } echo f(' ok ') . ',' . f(1);", "ok,no"),
    exec_instanceof => ("interface I { public function a(): string; } class C implements I { public function a(): string { return 'a'; } public function b(): string { return 'b'; } } function f(I $i): string { if ($i instanceof C) { return $i->b(); } return $i->a(); } echo f(new C());", "b"),
    exec_pdo_sqlite_memory => ("$pdo = new PDO('sqlite::memory:'); $pdo->exec('CREATE TABLE t (v TEXT)'); $st = $pdo->prepare('INSERT INTO t VALUES (?)'); $st->execute(['hi']); $q = $pdo->prepare('SELECT v FROM t'); $q->execute([]); $rows = $q->fetchAll(PDO::FETCH_ASSOC); $r = $rows[0]; if (is_array($r)) { echo strval($r['v']); }", "hi"),
    exec_json_encode => ("echo json_encode(['ok' => true]);", "{\"ok\":true}"),
    exec_strlen => ("echo (string) strlen('typephp');", "7"),
    exec_explode_implode => ("$p: string[] = explode('-', 'a-b-c'); echo implode(',', $p);", "a,b,c"),
    exec_str_starts_with => ("if (str_starts_with('typephp', 'type')) { echo '1'; }", "1"),
    exec_substr => ("echo substr('typephp', 4);", "php"),
    exec_trim => ("echo trim('  hi  ');", "hi"),
    exec_strtoupper_lower => ("echo strtoupper('a') . strtolower('B');", "Ab"),
    exec_htmlspecialchars => ("echo htmlspecialchars('<a>');", "&lt;a&gt;"),
    exec_array_push => ("$a: int[] = [1]; array_push($a, 2); echo (string) count($a);", "2"),
    exec_array_key_exists => ("$d: dict<string, int> = ['k' => 1]; if (array_key_exists('k', $d)) { echo 'found'; }", "found"),
    exec_neg_number => ("$x: int = -10; echo (string) ($x + 15);", "5"),
    exec_mod_op => ("echo (string) (10 % 3);", "1"),
    exec_div_op => ("echo (string) (10 / 2);", "5"),
    exec_logic_and_or => ("if (true && !false || false) { echo 'ok'; }", "ok"),
    exec_strict_cmp => ("if (1 === 1 && 1 !== 2) { echo 'strict'; }", "strict"),
    exec_const_top => ("const GREET: string = 'yo'; echo GREET;", "yo"),
    exec_const_inferred => ("const NUM = 99; echo (string) NUM;", "99"),
    exec_recursion => ("function fact(int $n): int { if ($n <= 1) { return 1; } return $n * fact($n - 1); } echo (string) fact(5);", "120"),
    exec_nested_shapes => ("type Inner = { v: int }; type Outer = { inner: Inner }; $o: Outer = ['inner' => ['v' => 8]]; $in = $o['inner']; echo (string) $in['v'];", "8"),
    exec_mutable_closure_capture => ("function make(int $base): (int) => int { return fn (int $x): int => $base + $x; } $f = make(10); echo (string) $f(5);", "15"),
    exec_class_default_field => ("class C { public int $v = 42; } $c = new C(); echo (string) $c->v;", "42"),
    exec_class_method_call_chain => ("class B { public function val(): string { return 'B'; } } class A { public function b(): B { return new B(); } } $a = new A(); echo $a->b()->val();", "B"),
    exec_cast_bool_int => ("$b: bool = (bool) 1; if ($b) { echo (string) (int) true; }", "1"),
    exec_empty_dict_check => ("$d: dict<string, int> = []; echo (string) count($d);", "0"),
    exec_nullable_return_null => ("function f(bool $ok): ?string { if ($ok) { return 'ok'; } return null; } echo f(false) ?? 'nil';", "nil"),
}

// --- 50 Multi-file `check_file` Graph Tests (Diamond, Cycles, Relative Paths, Cross-file Errors) ---
macro_rules! multifile_check_tests {
    ($($name:ident => ($files:expr, $expect_ok:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let dir = temp_dir(stringify!($name));
                for (rel, content) in $files {
                    let p = dir.join(rel);
                    if let Some(parent) = p.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    fs::write(p, content).unwrap();
                }
                let res = check_file(&dir.join("entry.tphp"));
                let _ = fs::remove_dir_all(&dir);
                assert_eq!(res.is_ok(), $expect_ok, "Unexpected check_file result: {:?}", res);
            }
        )*
    };
}

multifile_check_tests! {
    mf_missing_dep => ([("entry.tphp", "import { A } from './missing.tphp';")], false),
    mf_simple_import_ok => ([("a.tphp", "function f(): int { return 1; }"), ("entry.tphp", "import { f } from './a.tphp'; $x: int = f();")], true),
    mf_simple_import_err => ([("a.tphp", "function f(): int { return 1; }"), ("entry.tphp", "import { f } from './a.tphp'; $x: string = f();")], false),
    mf_diamond_graph_ok => ([("base.tphp", "type Id = int;"), ("left.tphp", "import { Id } from './base.tphp'; function l(Id $x): Id { return $x; }"), ("right.tphp", "import { Id } from './base.tphp'; function r(Id $x): Id { return $x; }"), ("entry.tphp", "import { l } from './left.tphp'; import { r } from './right.tphp'; $x: int = l(r(1));")], true),
    mf_cycle_does_not_hang => ([("a.tphp", "import './b.tphp'; $x: int = 1;"), ("b.tphp", "import './a.tphp'; $y: int = 2;"), ("entry.tphp", "import './a.tphp';")], true),
    mf_nested_subdirs_ok => ([("sub/lib.tphp", "class C { public function v(): int { return 5; } }"), ("entry.tphp", "import { C } from './sub/lib.tphp'; $c = new C(); $v: int = $c->v();")], true),
    mf_parent_dir_rel_ok => ([("shared/u.tphp", "type U = { id: int };"), ("app/entry.tphp", "import { U } from '../shared/u.tphp'; $u: U = ['id' => 1];"), ("entry.tphp", "import './app/entry.tphp';")], true),
    mf_cross_file_class_ok => ([("m.tphp", "class M { public string $s = 'hi'; }"), ("entry.tphp", "import { M } from './m.tphp'; $m = new M(); $s: string = $m->s;")], true),
    mf_cross_file_class_err => ([("m.tphp", "class M { public string $s = 'hi'; }"), ("entry.tphp", "import { M } from './m.tphp'; $m = new M(); $s: int = $m->s;")], false),
    mf_cross_file_iface_ok => ([("i.tphp", "interface I { public function ok(): bool; }"), ("entry.tphp", "import { I } from './i.tphp'; class C implements I { public function ok(): bool { return true; } }")], true),
    mf_cross_file_iface_err => ([("i.tphp", "interface I { public function ok(): bool; }"), ("entry.tphp", "import { I } from './i.tphp'; class C implements I {}")], false),
    mf_cross_file_enum_ok => ([("e.tphp", "enum E: string { case A = 'a'; }"), ("entry.tphp", "import { E } from './e.tphp'; $e: E = E::A;")], true),
    mf_cross_file_enum_err => ([("e.tphp", "enum E: string { case A = 'a'; }"), ("entry.tphp", "import { E } from './e.tphp'; $e: E = E::Missing;")], false),
    mf_cross_file_generic_ok => ([("g.tphp", "class Box<T> { private T $v; public function __construct(T $v) { $this->v = $v; } public function get(): T { return $this->v; } }"), ("entry.tphp", "import { Box } from './g.tphp'; $b = new Box<int>(1); $n: int = $b->get();")], true),
    mf_cross_file_generic_err => ([("g.tphp", "class Box<T> { private T $v; public function __construct(T $v) { $this->v = $v; } public function get(): T { return $this->v; } }"), ("entry.tphp", "import { Box } from './g.tphp'; $b = new Box<int>(1); $n: string = $b->get();")], false),
    mf_cross_file_shape_ok => ([("s.tphp", "type S = { id: int, name: string };"), ("entry.tphp", "import { S } from './s.tphp'; $s: S = ['id' => 1, 'name' => 'x'];")], true),
    mf_cross_file_shape_err => ([("s.tphp", "type S = { id: int, name: string };"), ("entry.tphp", "import { S } from './s.tphp'; $s: S = ['id' => 1];")], false),
    mf_require_tphp_ok => ([("lib.tphp", "function h(): string { return 'ok'; }"), ("entry.tphp", "require_once './lib.tphp'; $s: string = h();")], true),
    mf_require_php_ignored_in_dfs => ([("entry.tphp", "require_once './vendor/autoload.php'; $x: int = 1;")], true),
    mf_dep_lex_error_propagates => ([("bad.tphp", "$x = \"unterminated;"), ("entry.tphp", "import './bad.tphp';")], false),
    mf_dep_parse_error_propagates => ([("bad.tphp", "class {"), ("entry.tphp", "import './bad.tphp';")], false),
    mf_dep_type_error_propagates => ([("bad.tphp", "$x: int = 'bad';"), ("entry.tphp", "import './bad.tphp';")], false),
    mf_three_level_chain_ok => ([("c.tphp", "const V: int = 3;"), ("b.tphp", "import './c.tphp'; function getV(): int { return V; }"), ("a.tphp", "import { getV } from './b.tphp'; function twice(): int { return getV() * 2; }"), ("entry.tphp", "import { twice } from './a.tphp'; $r: int = twice();")], true),
    mf_three_level_chain_err => ([("c.tphp", "const V: string = '3';"), ("b.tphp", "import './c.tphp'; function getV(): int { return V; }"), ("entry.tphp", "import './b.tphp';")], false),
    mf_duplicate_import_ok => ([("a.tphp", "function f(): int { return 1; }"), ("entry.tphp", "import { f } from './a.tphp'; import { f } from './a.tphp'; $x: int = f();")], true),
    mf_namespace_use_ok => ([("a.tphp", "namespace Lib;\nclass Item { public int $id = 1; }"), ("entry.tphp", "import { Item } from './a.tphp'; use Lib\\Item; $i = new Item(); $id: int = $i->id;")], true),
    mf_private_across_files_err => ([("a.tphp", "class Secret { private int $p = 1; }"), ("entry.tphp", "import { Secret } from './a.tphp'; $s = new Secret(); $x = $s->p;")], false),
    mf_private_method_across_files_err => ([("a.tphp", "class Secret { private function p(): int { return 1; } }"), ("entry.tphp", "import { Secret } from './a.tphp'; $s = new Secret(); $x = $s->p();")], false),
    mf_generic_alias_across_files_ok => ([("a.tphp", "type Res<T> = { ok: bool, data: T };"), ("entry.tphp", "import { Res } from './a.tphp'; $r: Res<int> = ['ok' => true, 'data' => 5];")], true),
    mf_generic_alias_across_files_err => ([("a.tphp", "type Res<T> = { ok: bool, data: T };"), ("entry.tphp", "import { Res } from './a.tphp'; $r: Res<int> = ['ok' => true, 'data' => 'bad'];")], false),
    mf_fn_type_alias_across_files_ok => ([("a.tphp", "type FnT = (int) => string;"), ("entry.tphp", "import { FnT } from './a.tphp'; $f: FnT = fn (int $n): string => (string) $n;")], true),
    mf_fn_type_alias_across_files_err => ([("a.tphp", "type FnT = (int) => string;"), ("entry.tphp", "import { FnT } from './a.tphp'; $f: FnT = fn (int $n): int => $n;")], false),
    mf_union_alias_across_files_ok => ([("a.tphp", "type Id = int|string;"), ("entry.tphp", "import { Id } from './a.tphp'; $i: Id = 1; $i = 'abc';")], true),
    mf_union_alias_across_files_err => ([("a.tphp", "type Id = int|string;"), ("entry.tphp", "import { Id } from './a.tphp'; $i: Id = false;")], false),
    mf_interface_extends_across_files_ok => ([("a.tphp", "interface A { public function a(): int; }"), ("b.tphp", "import { A } from './a.tphp'; interface B extends A { public function b(): string; }"), ("entry.tphp", "import { B } from './b.tphp'; class C implements B { public function a(): int { return 1; } public function b(): string { return 'b'; } }")], true),
    mf_interface_extends_across_files_err => ([("a.tphp", "interface A { public function a(): int; }"), ("b.tphp", "import { A } from './a.tphp'; interface B extends A { public function b(): string; }"), ("entry.tphp", "import { B } from './b.tphp'; class C implements B { public function b(): string { return 'b'; } }")], false),
    mf_polymorphic_return_ok => ([("a.tphp", "interface I { public function n(): int; } class C implements I { public function n(): int { return 1; } }"), ("entry.tphp", "import { I, C } from './a.tphp'; function make(): I { return new C(); }")], true),
    mf_polymorphic_param_ok => ([("a.tphp", "interface I { public function n(): int; } class C implements I { public function n(): int { return 1; } }"), ("entry.tphp", "import { I, C } from './a.tphp'; function take(I $i): int { return $i->n(); } $x: int = take(new C());")], true),
    mf_generic_fn_inferred_across_files_ok => ([("a.tphp", "function first<T>(T[] $xs): T { return $xs[0]; }"), ("entry.tphp", "import { first } from './a.tphp'; $s: string = first(['a', 'b']);")], true),
    mf_generic_fn_inferred_across_files_err => ([("a.tphp", "function first<T>(T[] $xs): T { return $xs[0]; }"), ("entry.tphp", "import { first } from './a.tphp'; $n: int = first(['a', 'b']);")], false),
    mf_nullable_param_across_files_ok => ([("a.tphp", "function len(?string $s): int { if ($s === null) { return 0; } return strlen($s); }"), ("entry.tphp", "import { len } from './a.tphp'; $n: int = len(null);")], true),
    mf_nullable_param_across_files_err => ([("a.tphp", "function len(?string $s): int { if ($s === null) { return 0; } return strlen($s); }"), ("entry.tphp", "import { len } from './a.tphp'; $n: int = len(123);")], false),
    mf_pdo_repo_across_files_ok => ([("repo.tphp", "class Repo { private PDO $p; public function __construct() { $this->p = new PDO('sqlite::memory:'); } public function ping(): int { return $this->p->exec('SELECT 1'); } }"), ("entry.tphp", "import { Repo } from './repo.tphp'; $r = new Repo(); $n: int = $r->ping();")], true),
    mf_static_enum_param_across_files_ok => ([("e.tphp", "enum Mode: string { case Fast = 'fast'; }"), ("entry.tphp", "import { Mode } from './e.tphp'; function setMode(Mode $m): Mode { return $m; } $m: Mode = setMode(Mode::Fast);")], true),
    mf_static_enum_param_across_files_err => ([("e.tphp", "enum Mode: string { case Fast = 'fast'; }"), ("entry.tphp", "import { Mode } from './e.tphp'; function setMode(Mode $m): Mode { return $m; } $m: Mode = setMode('fast');")], false),
    mf_void_fn_across_files_ok => ([("v.tphp", "function logMsg(string $m): void { echo $m; }"), ("entry.tphp", "import { logMsg } from './v.tphp'; logMsg('ok');")], true),
    mf_void_fn_assigned_err => ([("v.tphp", "function logMsg(string $m): void { echo $m; }"), ("entry.tphp", "import { logMsg } from './v.tphp'; $x: int = logMsg('ok');")], false),
    mf_optional_shape_field_across_files_ok => ([("s.tphp", "type Opt = { a: int, b?: string };"), ("entry.tphp", "import { Opt } from './s.tphp'; $x: Opt = ['a' => 1]; $b: ?string = $x['b'];")], true),
    mf_optional_shape_field_mismatch_err => ([("s.tphp", "type Opt = { a: int, b?: string };"), ("entry.tphp", "import { Opt } from './s.tphp'; $x: Opt = ['a' => 1, 'b' => 2];")], false),
    mf_missing_entry_file_err => ([("other.tphp", "$x = 1;")], false),
}
