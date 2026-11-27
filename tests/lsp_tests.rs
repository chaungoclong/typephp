use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use typephp::lsp::{parse_json, Json, SymbolIndex};

#[test]
fn test_json_roundtrip_and_accessors() {
    let raw = r#"{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///test.tphp"},"position":{"line":3,"character":10}},"tags":["a",true,null]}"#;
    let parsed = parse_json(raw).expect("valid JSON");
    assert_eq!(parsed.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
    assert_eq!(parsed.get("id").and_then(|v| v.as_usize()), Some(1));
    assert_eq!(
        parsed
            .get("params")
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("line"))
            .and_then(|l| l.as_usize()),
        Some(3)
    );
    let compact = parsed.to_string_compact();
    let reparsed = parse_json(&compact).expect("reparsed JSON");
    assert_eq!(
        reparsed.get("method").and_then(|v| v.as_str()),
        Some("textDocument/hover")
    );

    let obj = Json::Obj(vec![
        ("msg".to_string(), Json::Str("hello \"world\"\n".to_string())),
        ("ok".to_string(), Json::Bool(true)),
    ]);
    let s = obj.to_string_compact();
    let p = parse_json(&s).expect("escaped JSON");
    assert_eq!(
        p.get("msg").and_then(|v| v.as_str()),
        Some("hello \"world\"\n")
    );
}

#[test]
fn test_symbol_index_defs_hover_and_references() {
    let src = r#"<?php
type UserId = int;

interface Identifiable {
    public function id(): UserId;
}

enum Role: string {
    case Admin = "admin";
    case Guest = "guest";
}

class User implements Identifiable {
    public UserId $uid;
    public function __construct(UserId $uid) {
        $this->uid = $uid;
    }
    public function id(): UserId {
        return $this->uid;
    }
}

function greet(User $user): string {
    $greeting = "Hi";
    return $greeting;
}

$u = new User(1);
$msg = greet($u);
"#;

    let idx = SymbolIndex::build(src);

    // Type alias UserId: 1 def + multiple references (lines 4, 13, 14, 17 -> 5 occurrences total)
    let user_id_def = idx
        .defs
        .iter()
        .find(|d| d.name == "UserId" && d.kind == "type")
        .expect("UserId type def");
    let user_id_refs = idx.references_for(user_id_def.id);
    assert!(
        user_id_refs.len() >= 4,
        "expected >=4 references for UserId, got {}",
        user_id_refs.len()
    );

    // Hover on `UserId` at line 1 (0-based), col 5 (`type UserId = int;`)
    let found_type = idx.find_at(1, 6).expect("find UserId at def");
    assert_eq!(found_type.name, "UserId");
    assert_eq!(found_type.kind, "type");

    // Hover on `Identifiable` in `class User implements Identifiable` (line 12, 0-based)
    let line12 = src.lines().nth(12).unwrap();
    let ident_col = line12.find("Identifiable").unwrap();
    let found_iface = idx.find_at(12, ident_col + 2).expect("find Identifiable");
    assert_eq!(found_iface.name, "Identifiable");
    assert_eq!(found_iface.kind, "interface");

    // Enum Role
    let role_def = idx
        .defs
        .iter()
        .find(|d| d.name == "Role" && d.kind == "enum")
        .expect("Role enum def");
    assert!(!idx.references_for(role_def.id).is_empty());

    // Class User: def + param type in greet + `new User(1)`
    let user_class_def = idx
        .defs
        .iter()
        .find(|d| d.name == "User" && d.kind == "class")
        .expect("User class def");
    let user_refs = idx.references_for(user_class_def.id);
    assert!(
        user_refs.len() >= 3,
        "expected >=3 references for User class, got {}",
        user_refs.len()
    );

    // Function greet: def + call `greet($u)`
    let greet_def = idx
        .defs
        .iter()
        .find(|d| d.name == "greet" && d.kind == "function")
        .expect("greet function def");
    let greet_refs = idx.references_for(greet_def.id);
    assert_eq!(greet_refs.len(), 2, "expected def + call for greet");

    // Variable $greeting inside greet
    let greeting_def = idx
        .defs
        .iter()
        .find(|d| d.name == "$greeting" && d.kind == "variable")
        .expect("$greeting var def");
    assert_eq!(idx.references_for(greeting_def.id).len(), 2);

    // Top-level variable $u: `$u = new User(1);` + `greet($u);`
    let u_def = idx
        .defs
        .iter()
        .find(|d| d.name == "$u" && d.kind == "variable")
        .expect("$u var def");
    assert_eq!(idx.references_for(u_def.id).len(), 2);
}

fn send_lsp_msg(stdin: &mut std::process::ChildStdin, json_body: &str) {
    let header = format!("Content-Length: {}\r\n\r\n", json_body.len());
    stdin.write_all(header.as_bytes()).unwrap();
    stdin.write_all(json_body.as_bytes()).unwrap();
    stdin.flush().unwrap();
}

fn read_lsp_msg(reader: &mut BufReader<std::process::ChildStdout>) -> Json {
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).unwrap();
        assert!(n > 0, "unexpected EOF from LSP server");
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().unwrap();
        }
    }
    assert!(content_length > 0, "missing Content-Length");
    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    parse_json(&s).expect("valid LSP JSON response")
}

#[test]
fn test_lsp_stdio_end_to_end() {
    let bin = env!("CARGO_BIN_EXE_tphpc");
    let mut child = Command::new(bin)
        .arg("--lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tphpc --lsp");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // 1. initialize
    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init_resp = read_lsp_msg(&mut reader);
    assert_eq!(init_resp.get("id").and_then(|v| v.as_usize()), Some(1));
    let caps = init_resp
        .get("result")
        .and_then(|r| r.get("capabilities"))
        .expect("capabilities");
    assert!(caps.get("hoverProvider").is_some());
    assert!(caps.get("definitionProvider").is_some());
    assert!(caps.get("referencesProvider").is_some());

    // 2. initialized + didOpen
    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
    let doc_text = "<?php\nfunction add(int $a, int $b): int {\n    return $a + $b;\n}\n$sum = add(2, 3);\n";
    let did_open = Json::Obj(vec![
        ("jsonrpc".to_string(), Json::Str("2.0".to_string())),
        (
            "method".to_string(),
            Json::Str("textDocument/didOpen".to_string()),
        ),
        (
            "params".to_string(),
            Json::Obj(vec![(
                "textDocument".to_string(),
                Json::Obj(vec![
                    ("uri".to_string(), Json::Str("file:///sample.tphp".to_string())),
                    ("languageId".to_string(), Json::Str("typephp".to_string())),
                    ("version".to_string(), Json::Num(1.0)),
                    ("text".to_string(), Json::Str(doc_text.to_string())),
                ]),
            )]),
        ),
    ]);
    send_lsp_msg(&mut stdin, &did_open.to_string_compact());

    // Expect publishDiagnostics notification
    let diag_msg = read_lsp_msg(&mut reader);
    assert_eq!(
        diag_msg.get("method").and_then(|v| v.as_str()),
        Some("textDocument/publishDiagnostics")
    );

    // 3. hover on `add` at line 4, col 8 (`$sum = add(2, 3);`)
    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///sample.tphp"},"position":{"line":4,"character":8}}}"#,
    );
    let hover_resp = read_lsp_msg(&mut reader);
    assert_eq!(hover_resp.get("id").and_then(|v| v.as_usize()), Some(2));
    let hover_val = hover_resp
        .get("result")
        .and_then(|r| r.get("contents"))
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .expect("hover contents");
    assert!(
        hover_val.contains("function add"),
        "unexpected hover value: {}",
        hover_val
    );

    // 4. definition on `add` at line 4, col 8 -> line 1
    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///sample.tphp"},"position":{"line":4,"character":8}}}"#,
    );
    let def_resp = read_lsp_msg(&mut reader);
    let def_line = def_resp
        .get("result")
        .and_then(|r| r.get("range"))
        .and_then(|rg| rg.get("start"))
        .and_then(|st| st.get("line"))
        .and_then(|l| l.as_usize());
    assert_eq!(def_line, Some(1));

    // 4b. definition on `add` at its own declaration (line 1, col 10) -> returns usage ref on line 4
    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":6,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///sample.tphp"},"position":{"line":1,"character":10}}}"#,
    );
    let def_at_decl = read_lsp_msg(&mut reader);
    let decl_refs = match def_at_decl.get("result") {
        Some(Json::Arr(arr)) => arr,
        other => panic!("expected array of refs when clicking on definition, got {:?}", other),
    };
    assert_eq!(decl_refs.len(), 1);
    assert_eq!(
        decl_refs[0]
            .get("range")
            .and_then(|rg| rg.get("start"))
            .and_then(|st| st.get("line"))
            .and_then(|l| l.as_usize()),
        Some(4)
    );

    // 5. references on `$a` at line 2, col 11 (`return $a + $b;`) -> 2 refs
    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"textDocument/references","params":{"textDocument":{"uri":"file:///sample.tphp"},"position":{"line":2,"character":12},"context":{"includeDeclaration":true}}}"#,
    );
    let refs_resp = read_lsp_msg(&mut reader);
    let refs_arr = match refs_resp.get("result") {
        Some(Json::Arr(arr)) => arr,
        other => panic!("expected array of references, got {:?}", other),
    };
    assert_eq!(refs_arr.len(), 2);

    // 6. shutdown + exit
    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":5,"method":"shutdown","params":null}"#,
    );
    let shutdown_resp = read_lsp_msg(&mut reader);
    assert_eq!(shutdown_resp.get("id").and_then(|v| v.as_usize()), Some(5));

    send_lsp_msg(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"exit","params":null}"#,
    );
    drop(stdin);
    let status = child.wait().expect("wait child");
    assert!(status.success(), "LSP server exited with {:?}", status);
}

#[test]
fn test_native_php_classes_and_functions() {
    let src = r#"<?php
const MAX_LIMIT: int = 50;
const APP_TAG = "v1";

class AppError extends Exception {}

function openDb(string $dsn): PDO {
    $db = new PDO($dsn);
    $mode = PDO::FETCH_ASSOC;
    $eol = PHP_EOL;
    $stmt = $db->prepare("SELECT 1");
    $rows = $stmt->fetchAll($mode);
    if ($db instanceof PDO) {
        $len = strlen($dsn);
        $has = array_key_exists("a", ["a" => 1]);
    }
    $dt = new DateTimeImmutable("now");
    $custom = new CustomNativeClass();
    $lim = MAX_LIMIT;
    return $db;
}
"#;
    let idx = SymbolIndex::build(src);

    // User typed constant MAX_LIMIT shows value
    let line1 = src.lines().nth(1).unwrap();
    let max_col = line1.find("MAX_LIMIT").unwrap();
    let max_def = idx.find_at(1, max_col + 2).expect("find MAX_LIMIT");
    assert_eq!(max_def.name, "MAX_LIMIT");
    assert_eq!(max_def.kind, "constant");
    assert_eq!(max_def.detail, "const MAX_LIMIT: int = 50");
    assert_eq!(idx.references_for(max_def.id).len(), 2);

    // User untyped constant APP_TAG shows value
    let line2 = src.lines().nth(2).unwrap();
    let tag_col = line2.find("APP_TAG").unwrap();
    let tag_def = idx.find_at(2, tag_col + 2).expect("find APP_TAG");
    assert_eq!(tag_def.detail, "const APP_TAG = \"v1\"");

    // Exception in `extends Exception` (line 4)
    let line4 = src.lines().nth(4).unwrap();
    let exc_col = line4.find("Exception").unwrap();
    let exc_def = idx.find_at(4, exc_col + 2).expect("find Exception");
    assert_eq!(exc_def.name, "Exception");
    assert_eq!(exc_def.kind, "class");
    assert_eq!(exc_def.span.line, 0);

    // PDO across return type (line 6), new PDO (line 7), PDO::FETCH_ASSOC (line 8), instanceof PDO (line 12)
    let line6 = src.lines().nth(6).unwrap();
    let pdo_col6 = line6.find("PDO").unwrap();
    let pdo_def = idx.find_at(6, pdo_col6 + 1).expect("find PDO on return type");
    assert_eq!(pdo_def.name, "PDO");
    assert!(pdo_def.detail.contains("PHP built-in"));
    assert_eq!(idx.references_for(pdo_def.id).len(), 4);

    // PDO::FETCH_ASSOC constant (line 8) shows value = 2
    let line8 = src.lines().nth(8).unwrap();
    let fetch_col = line8.find("FETCH_ASSOC").unwrap();
    let fetch_def = idx
        .find_at(8, fetch_col + 2)
        .expect("find PDO::FETCH_ASSOC");
    assert_eq!(fetch_def.name, "FETCH_ASSOC");
    assert_eq!(fetch_def.kind, "constant");
    assert!(
        fetch_def.detail.contains("= 2"),
        "expected PDO::FETCH_ASSOC value, got {}",
        fetch_def.detail
    );

    // Global PHP constant PHP_EOL (line 9) shows value
    let line9 = src.lines().nth(9).unwrap();
    let eol_col = line9.find("PHP_EOL").unwrap();
    let eol_def = idx.find_at(9, eol_col + 2).expect("find PHP_EOL");
    assert_eq!(eol_def.name, "PHP_EOL");
    assert_eq!(eol_def.kind, "constant");
    assert!(
        eol_def.detail.contains("= "),
        "expected PHP_EOL value, got {}",
        eol_def.detail
    );

    // $db->prepare(...) method on PDO (line 10)
    let line10 = src.lines().nth(10).unwrap();
    let prep_col = line10.find("prepare").unwrap();
    let prep_def = idx.find_at(10, prep_col + 2).expect("find PDO::prepare");
    assert_eq!(prep_def.name, "prepare");
    assert_eq!(prep_def.kind, "method");
    assert!(prep_def.detail.contains("PDO::prepare"));

    // $stmt->fetchAll(...) inferred from $db->prepare() -> PDOStatement (line 11)
    let line11 = src.lines().nth(11).unwrap();
    let fa_col = line11.find("fetchAll").unwrap();
    let fa_def = idx
        .find_at(11, fa_col + 2)
        .expect("find PDOStatement::fetchAll");
    assert_eq!(fa_def.name, "fetchAll");
    assert_eq!(fa_def.kind, "method");
    assert!(fa_def.detail.contains("PDOStatement::fetchAll"));

    // Verify instanceof PDO span specifically (line 12)
    let line12 = src.lines().nth(12).unwrap();
    let pdo_col12 = line12.find("PDO").unwrap();
    let pdo_inst = idx.find_at(12, pdo_col12 + 1).expect("find PDO in instanceof");
    assert_eq!(pdo_inst.name, "PDO");

    // Native PHP function `strlen` (line 13)
    let line13 = src.lines().nth(13).unwrap();
    let strlen_col = line13.find("strlen").unwrap();
    let strlen_def = idx.find_at(13, strlen_col + 2).expect("find strlen");
    assert_eq!(strlen_def.name, "strlen");
    assert_eq!(strlen_def.kind, "function");

    // Native PHP function `array_key_exists` discovered via Reflection (line 14)
    let line14 = src.lines().nth(14).unwrap();
    let ake_col = line14.find("array_key_exists").unwrap();
    let ake_def = idx
        .find_at(14, ake_col + 2)
        .expect("find array_key_exists via PHP Reflection");
    assert_eq!(ake_def.name, "array_key_exists");
    assert_eq!(ake_def.kind, "function");

    // DateTimeImmutable (line 16)
    let line16 = src.lines().nth(16).unwrap();
    let dt_col = line16.find("DateTimeImmutable").unwrap();
    let dt_def = idx.find_at(16, dt_col + 2).expect("find DateTimeImmutable");
    assert_eq!(dt_def.name, "DateTimeImmutable");

    // Fallback external/PHP class `CustomNativeClass` (line 17)
    let line17 = src.lines().nth(17).unwrap();
    let custom_col = line17.find("CustomNativeClass").unwrap();
    let custom_def = idx
        .find_at(17, custom_col + 2)
        .expect("find CustomNativeClass");
    assert_eq!(custom_def.name, "CustomNativeClass");
    assert_eq!(custom_def.kind, "class");
}
