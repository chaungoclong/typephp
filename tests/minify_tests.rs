use typephp::codegen::{minify_php, Codegen};
use typephp::lexer::Lexer;
use typephp::minify::mangle_programs;
use typephp::parser::Parser;

// ============================================================================
// Minifier & Mangler Tests (MC/DC) — 100+ test cases
// ============================================================================

macro_rules! minify_php_eq_tests {
    ($($name:ident => ($input:expr, $expected:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(minify_php($input), $expected);
            }
        )*
    };
}

macro_rules! mangle_checks {
    ($($name:ident => ($src:expr, $mangle_fns:expr, $must_contain:expr, $must_not_contain:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let tokens = Lexer::new($src).tokenize().unwrap();
                let mut prog = Parser::new(tokens).parse_program().unwrap();
                mangle_programs(std::slice::from_mut(&mut prog), $mangle_fns);
                let out = minify_php(&Codegen::new().emit_program(&prog));
                assert!(out.contains($must_contain), "Missing {:?} in:\n{}", $must_contain, out);
                if !$must_not_contain.is_empty() {
                    assert!(!out.contains($must_not_contain), "Unexpected {:?} in:\n{}", $must_not_contain, out);
                }
            }
        )*
    };
}

// --- 50 `minify_php` MC/DC & Edge Case Tests ---
minify_php_eq_tests! {
    min_php_word_word => ("<?php\necho   $x;", "<?php echo $x;"),
    min_php_word_symbol => ("$a  +  $b;", "$a+$b;"),
    min_php_digit_dot => ("echo 1 . 'a';", "echo 1 .'a';"),
    min_php_dot_digit => ("echo 'a' . 1;", "echo 'a'. 1;"),
    min_php_question_question => ("$x = $a ? ?int : 0;", "$x=$a? ?int:0;"),
    min_php_null_coal_no_space => ("$x = $a ?? $b;", "$x=$a??$b;"),
    min_php_strip_line_comment => ("$a = 1; // comment\n$b = 2;", "$a=1;$b=2;"),
    min_php_strip_hash_comment => ("$a = 1; # hash\n$b = 2;", "$a=1;$b=2;"),
    min_php_strip_block_comment => ("$a = /* c */ 1;", "$a=1;"),
    min_php_strip_phpdoc => ("/** @var int $x */\n$x = 1;", "$x=1;"),
    min_php_preserve_single_string_spaces => ("$s = 'a   b';", "$s='a   b';"),
    min_php_preserve_double_string_spaces => ("$s = \"a   b\";", "$s=\"a   b\";"),
    min_php_preserve_comment_in_single_str => ("$s = '// not comment';", "$s='// not comment';"),
    min_php_preserve_comment_in_double_str => ("$s = \"/* not comment */\";", "$s=\"/* not comment */\";"),
    min_php_escaped_single_quote => ("$s = 'it\\'s   ok';", "$s='it\\'s   ok';"),
    min_php_escaped_double_quote => ("$s = \"say \\\"hi   there\\\"\";", "$s=\"say \\\"hi   there\\\"\";"),
    min_php_return_var => ("return   $res;", "return $res;"),
    min_php_return_void => ("return ;", "return;"),
    min_php_new_class => ("$x = new   Foo ( ) ;", "$x=new Foo();"),
    min_php_fn_decl => ("function   foo ( int   $a ) : int { return   $a ; }", "function foo(int $a):int{return $a;}"),
    min_php_arrow_fn => ("fn ( int $x ) : int => $x * 2", "fn(int $x):int=>$x*2"),
    min_php_if_else => ("if ( $a === 1 ) { echo 1 ; } else { echo 2 ; }", "if($a===1){echo 1;}else{echo 2;}"),
    min_php_elseif => ("if ($a) { echo 1; } elseif ($b) { echo 2; }", "if($a){echo 1;}elseif($b){echo 2;}"),
    min_php_while => ("while ( $i < 10 ) { $i = $i + 1 ; }", "while($i<10){$i=$i+1;}"),
    min_php_foreach => ("foreach ( $xs as $k => $v ) { echo $k ; }", "foreach($xs as $k=>$v){echo $k;}"),
    min_php_namespace_semicolon => ("namespace   App\\Core ;", "namespace App\\Core;"),
    min_php_namespace_brace => ("namespace   App\\Core\n{\n  class A {}\n}", "namespace App\\Core{class A{}}"),
    min_php_use_stmt => ("use   App\\Core\\A ;", "use App\\Core\\A;"),
    min_php_require_once => ("require_once   __DIR__ . '/a.php' ;", "require_once __DIR__.'/a.php';"),
    min_php_class_extends => ("class   B   extends   A   implements   I1 , I2 { }", "class B extends A implements I1,I2{}"),
    min_php_enum_case => ("enum   E : string { case   A = 'a' ; }", "enum E:string{case A='a';}"),
    min_php_const => ("const   PI = 3.14 ;", "const PI=3.14;"),
    min_php_cast => ("$x = ( string )   $y ;", "$x=(string)$y;"),
    min_php_instanceof => ("$b = $x   instanceof   Foo ;", "$b=$x instanceof Foo;"),
    min_php_static_access => ("$x = Foo :: BAR ;", "$x=Foo::BAR;"),
    min_php_member_access => ("$x = $this -> prop -> call ( ) ;", "$x=$this->prop->call();"),
    min_php_array_lit => ("$a = [ 1 , 2 , 3 ] ;", "$a=[1,2,3];"),
    min_php_assoc_array => ("$a = [ 'k' => 1 , 'm' => 2 ] ;", "$a=['k'=>1,'m'=>2];"),
    min_php_declare_strict => ("<?php\n\ndeclare ( strict_types = 1 ) ;", "<?php declare(strict_types=1);"),
    min_php_multiple_comments => ("// a\n// b\n/* c */$x = 1;", "$x=1;"),
    min_php_crlf_newlines => ("$a = 1;\r\n$b = 2;\r\n", "$a=1;$b=2;"),
    min_php_tabs_collapse => ("$a\t=\t1\t+\t2;", "$a=1+2;"),
    min_php_unary_neg => ("$x = - 5;", "$x=-5;"),
    min_php_unary_not => ("$x = ! $y;", "$x=!$y;"),
    min_php_string_after_word => ("echo   'hi' ;", "echo 'hi';"),
    min_php_double_string_after_word => ("echo   \"hi\" ;", "echo \"hi\";"),
    min_php_float_preserved => ("$x = 3.14;", "$x=3.14;"),
    min_php_empty_input => ("   \n\t ", ""),
    min_php_only_comments => ("// hi\n/* there */", ""),
    min_php_trailing_space_removed => ("$x = 1;   \n  ", "$x=1;"),
}

// --- 30 Mangling Tests (Variables, Params, Foreach, ArrowFn, Superglobals, Functions) ---
mangle_checks! {
    mangle_fn_params_and_local => ("function f(int $longParam): int { $longLocal = $longParam + 1; return $longLocal; }", false, "function f(int $a):int{$b=$a+1;return $b;}", "longParam"),
    mangle_top_level_fn_enabled => ("function myHelper(int $v): int { return $v; } $x = myHelper(1);", true, "function _a(int $a):int{return $a;}", "myHelper"),
    mangle_top_level_fn_disabled => ("function myHelper(int $v): int { return $v; } $x = myHelper(1);", false, "function myHelper(int $a):int", "_a"),
    mangle_preserves_this => ("class C { public int $v = 1; public function get(): int { return $this->v; } }", false, "return $this->v;", "$a->v"),
    mangle_preserves_prop_name => ("class C { public string $myProp = ''; public function set(string $val): void { $this->myProp = $val; } }", false, "$this->myProp=$a;", ""),
    mangle_preserves_server => ("$m = $_SERVER['REQUEST_METHOD'] ?? 'GET';", false, "$a=$_SERVER['REQUEST_METHOD']??'GET';", ""),
    mangle_preserves_get => ("$g = $_GET['id'] ?? '0';", false, "$a=$_GET['id']??'0';", ""),
    mangle_preserves_post => ("$p = $_POST['name'] ?? '';", false, "$a=$_POST['name']??'';", ""),
    mangle_preserves_cookie => ("$c = $_COOKIE['sid'] ?? '';", false, "$a=$_COOKIE['sid']??'';", ""),
    mangle_preserves_files => ("$f = $_FILES['up'] ?? null;", false, "$a=$_FILES['up']??null;", ""),
    mangle_preserves_env => ("$e = $_ENV['APP'] ?? '';", false, "$a=$_ENV['APP']??'';", ""),
    mangle_preserves_request => ("$r = $_REQUEST['q'] ?? '';", false, "$a=$_REQUEST['q']??'';", ""),
    mangle_preserves_session => ("$s = $_SESSION['uid'] ?? '';", false, "$a=$_SESSION['uid']??'';", ""),
    mangle_preserves_globals => ("$g = $GLOBALS['cfg'] ?? null;", false, "$a=$GLOBALS['cfg']??null;", ""),
    mangle_foreach_val => ("$items = [1, 2]; foreach ($items as $itemVal) { echo $itemVal; }", false, "foreach($a as $b){echo $b;}", "itemVal"),
    mangle_foreach_kv => ("$map = ['a' => 1]; foreach ($map as $keyName => $valName) { echo $keyName; }", false, "foreach($a as $b=>$c){echo $b;}", "keyName"),
    mangle_arrow_captures_outer => ("function f(int $outerVar): (int) => int { return fn (int $innerVar): int => $outerVar + $innerVar; }", false, "fn(int $b):int=>$a+$b", "outerVar"),
    mangle_ctor_params => ("class C { private int $x; public function __construct(int $initVal) { $this->x = $initVal; } }", false, "__construct(int $a){$this->x=$a;}", "initVal"),
    mangle_multiple_fns_distinct_names => ("function f1(): int { return 1; } function f2(): int { return 2; } $a = f1() + f2();", true, "_a()+_b()", "f1"),
    mangle_class_name_preserved => ("class MyService { public function run(): int { return 1; } }", true, "class MyService", ""),
    mangle_interface_name_preserved => ("interface MyIface { public function run(): void; }", true, "interface MyIface", ""),
    mangle_enum_name_preserved => ("enum MyEnum: string { case Ok = 'ok'; }", true, "enum MyEnum:string", ""),
    mangle_method_name_preserved => ("class S { public function handleReq(): void {} }", true, "public function handleReq():void", ""),
    mangle_while_cond_and_body => ("$counter = 0; while ($counter < 3) { $counter = $counter + 1; }", false, "while($a<3){$a=$a+1;}", "counter"),
    mangle_if_else_branches => ("$flag = true; if ($flag) { $res = 1; } else { $res = 2; }", false, "if($a){$b=1;}else{$b=2;}", "flag"),
    mangle_cast_expr => ("$raw = '10'; $num = (int) $raw;", false, "$b=(int)$a;", "raw"),
    mangle_index_expr => ("$arr = [10]; $first = $arr[0];", false, "$b=$a[0];", "first"),
    mangle_new_args => ("class B { public function __construct(int $v) {} } $val = 5; $obj = new B($val);", false, "$b=new B($a);", "val"),
    mangle_fn_default_arg => ("function f(int $first = 1, int $second = 2): int { return $first + $second; }", false, "function f(int $a=1,int $b=2):int", "second"),
    mangle_27_vars_rollover_aa => ("function f(): void { $v1=1;$v2=1;$v3=1;$v4=1;$v5=1;$v6=1;$v7=1;$v8=1;$v9=1;$v10=1;$v11=1;$v12=1;$v13=1;$v14=1;$v15=1;$v16=1;$v17=1;$v18=1;$v19=1;$v20=1;$v21=1;$v22=1;$v23=1;$v24=1;$v25=1;$v26=1;$v27=1; }", false, "$aa=1;", "v27"),
}

// --- 22 Embedded HTML/CSS/JS Minification Tests (MC/DC) ---
fn run_web_min(html: &str) -> String {
    let src = format!("$h = '{}';", html.replace('\'', "\\'"));
    let tokens = Lexer::new(&src).tokenize().unwrap();
    let mut prog = Parser::new(tokens).parse_program().unwrap();
    mangle_programs(std::slice::from_mut(&mut prog), false);
    minify_php(&Codegen::new().emit_program(&prog))
}

#[test]
fn mcdc_web_script_before_style() {
    let out = run_web_min("<div><script>const x = 1;</script><style>a { color: red; }</style></div>");
    assert!(out.contains("const x=1;"));
    assert!(out.contains("a{color:red;}"));
}

#[test]
fn mcdc_web_style_before_script() {
    let out = run_web_min("<div><style>a { color: blue; }</style><script>const y = 2;</script></div>");
    assert!(out.contains("a{color:blue;}"));
    assert!(out.contains("const y=2;"));
}

#[test]
fn mcdc_web_only_script() {
    let out = run_web_min("<div><script>let z = 3;</script></div>");
    assert!(out.contains("let z=3;"));
}

#[test]
fn mcdc_web_only_style() {
    let out = run_web_min("<div><style>p { margin: 0; }</style></div>");
    assert!(out.contains("p{margin:0;}"));
}

#[test]
fn mcdc_web_plain_html_only() {
    let out = run_web_min("<div>   <span>Hello</span>   </div>");
    assert!(out.contains("<div><span>Hello</span></div>"));
}

macro_rules! web_min_tests {
    ($($name:ident => ($html:expr, $contains:expr, $not_contains:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let out = run_web_min($html);
                assert!(out.contains($contains), "Expected {:?} in:\n{}", $contains, out);
                if !$not_contains.is_empty() {
                    assert!(!out.contains($not_contains), "Unexpected {:?} in:\n{}", $not_contains, out);
                }
            }
        )*
    };
}

web_min_tests! {
    web_strip_html_comment => ("<div><!-- secret --><p>Hi</p></div>", "<div><p>Hi</p></div>", "secret"),
    web_strip_multiline_html_comment => ("<div><!--\nline1\nline2\n--><span>Ok</span></div>", "<div><span>Ok</span></div>", "line1"),
    web_collapse_between_tags => ("<div>\n    <span>A</span>\n    <span>B</span>\n</div>", "<div><span>A</span><span>B</span></div>", "\n"),
    web_doctype_html => ("<!DOCTYPE html>\n<html>\n  <body>Hi</body>\n</html>", "<!DOCTYPE html><html><body>Hi</body></html>", "\n"),
    web_article_tag => ("<article>\n  <h2>Title</h2>\n</article>", "<article><h2>Title</h2></article>", "\n"),
    web_section_tag => ("<section>\n  <p>Text</p>\n</section>", "<section><p>Text</p></section>", "\n"),
    web_css_strip_comment => ("<style>/* css */ body { margin: 0; }</style>", "body{margin:0;}", "/* css */"),
    web_css_collapse_spaces => ("<style>.btn  {  padding : 4px ; color : #fff ; }</style>", ".btn{padding:4px;color:#fff;}", "  "),
    web_css_keep_space_selectors => ("<style>div p { color: red; }</style>", "div p{color:red;}", "divp"),
    web_js_strip_line_comment => ("<script>// cmt\nconst a = 1;</script>", "const a=1;", "// cmt"),
    web_js_strip_block_comment => ("<script>/* blk */ let b = 2;</script>", "let b=2;", "/* blk */"),
    web_js_keep_double_string => ("<script>const s = \"a   b\";</script>", "const s=\"a   b\";", ""),
    web_js_keep_template_string => ("<script>const t = `x   y`;</script>", "const t=`x   y`;", ""),
    web_js_keep_keyword_space => ("<script>async function go() { return 1; }</script>", "async function go(){return 1;}", "asyncfunction"),
    web_multiple_script_blocks => ("<div><script>const a = 1;</script><script>const b = 2;</script></div>", "const a=1;</script><script>const b=2;", ""),
    web_multiple_style_blocks => ("<div><style>a { color: red; }</style><style>b { color: blue; }</style></div>", "a{color:red;}</style><style>b{color:blue;}", ""),
    web_non_html_string_untouched => ("Plain   text   with   spaces", "Plain   text   with   spaces", ""),
}
