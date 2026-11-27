use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::ast::*;
use crate::error::Span;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::{check_with_warnings};

// ============================================================================
// 1. Minimal Zero-Dependency JSON Parser & Writer
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        if let Json::Obj(entries) = self {
            entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Json::Str(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        if let Json::Num(n) = self {
            if *n >= 0.0 {
                return Some(*n as usize);
            }
        }
        None
    }

    pub fn to_string_compact(&self) -> String {
        match self {
            Json::Null => "null".to_string(),
            Json::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Json::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Json::Str(s) => format!("\"{}\"", escape_json_str(s)),
            Json::Arr(items) => {
                let parts: Vec<String> = items.iter().map(|i| i.to_string_compact()).collect();
                format!("[{}]", parts.join(","))
            }
            Json::Obj(entries) => {
                let parts: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", escape_json_str(k), v.to_string_compact()))
                    .collect();
                format!("{{{}}}", parts.join(","))
            }
        }
    }
}

fn escape_json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

pub fn parse_json(input: &str) -> Result<Json, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0usize;
    parse_json_val(&chars, &mut pos)
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_json_val(chars: &[char], pos: &mut usize) -> Result<Json, String> {
    skip_ws(chars, pos);
    if *pos >= chars.len() {
        return Err("Unexpected EOF in JSON".into());
    }
    match chars[*pos] {
        '{' => {
            *pos += 1;
            let mut entries = Vec::new();
            skip_ws(chars, pos);
            if *pos < chars.len() && chars[*pos] == '}' {
                *pos += 1;
                return Ok(Json::Obj(entries));
            }
            loop {
                skip_ws(chars, pos);
                let key = parse_json_str(chars, pos)?;
                skip_ws(chars, pos);
                if chars.get(*pos) != Some(&':') {
                    return Err("Expected ':' in JSON object".into());
                }
                *pos += 1;
                let val = parse_json_val(chars, pos)?;
                entries.push((key, val));
                skip_ws(chars, pos);
                match chars.get(*pos) {
                    Some(&',') => *pos += 1,
                    Some(&'}') => {
                        *pos += 1;
                        break;
                    }
                    _ => return Err("Expected ',' or '}' in JSON object".into()),
                }
            }
            Ok(Json::Obj(entries))
        }
        '[' => {
            *pos += 1;
            let mut items = Vec::new();
            skip_ws(chars, pos);
            if *pos < chars.len() && chars[*pos] == ']' {
                *pos += 1;
                return Ok(Json::Arr(items));
            }
            loop {
                items.push(parse_json_val(chars, pos)?);
                skip_ws(chars, pos);
                match chars.get(*pos) {
                    Some(&',') => *pos += 1,
                    Some(&']') => {
                        *pos += 1;
                        break;
                    }
                    _ => return Err("Expected ',' or ']' in JSON array".into()),
                }
            }
            Ok(Json::Arr(items))
        }
        '"' => Ok(Json::Str(parse_json_str(chars, pos)?)),
        't' => {
            *pos += 4;
            Ok(Json::Bool(true))
        }
        'f' => {
            *pos += 5;
            Ok(Json::Bool(false))
        }
        'n' => {
            *pos += 4;
            Ok(Json::Null)
        }
        c if c == '-' || c.is_ascii_digit() => {
            let start = *pos;
            while *pos < chars.len()
                && (chars[*pos].is_ascii_digit()
                    || matches!(chars[*pos], '-' | '+' | '.' | 'e' | 'E'))
            {
                *pos += 1;
            }
            let s: String = chars[start..*pos].iter().collect();
            let n: f64 = s.parse().map_err(|_| format!("Invalid JSON number: {}", s))?;
            Ok(Json::Num(n))
        }
        other => Err(format!("Unexpected JSON char '{}'", other)),
    }
}

fn parse_json_str(chars: &[char], pos: &mut usize) -> Result<String, String> {
    if chars.get(*pos) != Some(&'"') {
        return Err("Expected '\"'".into());
    }
    *pos += 1;
    let mut out = String::new();
    while *pos < chars.len() {
        let ch = chars[*pos];
        *pos += 1;
        if ch == '"' {
            return Ok(out);
        } else if ch == '\\' && *pos < chars.len() {
            let esc = chars[*pos];
            *pos += 1;
            match esc {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(ch);
        }
    }
    Err("Unterminated JSON string".into())
}

// ============================================================================
// 2. Symbol Index (Definitions, References, Hover Signatures)
// ============================================================================

#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub id: usize,
    pub name: String,
    pub kind: &'static str,
    pub detail: String,
    pub span: Span,
    pub len: usize,
}

#[derive(Debug, Clone)]
pub struct SymbolOcc {
    pub span: Span,
    pub len: usize,
    pub def_id: usize,
}

#[derive(Debug, Clone)]
pub struct SymbolIndex {
    pub defs: Vec<SymbolDef>,
    pub occs: Vec<SymbolOcc>,
    imports: HashMap<usize, String>,
}

#[derive(Debug, Clone)]
struct PhpFnMeta {
    name: String,
    sig: String,
    ret: String,
}

#[derive(Debug, Clone)]
struct PhpClassMeta {
    name: String,
    kind: &'static str,
    summary: String,
    methods: HashMap<String, PhpFnMeta>,
    consts: HashMap<String, String>,
}

struct PhpCatalog {
    functions: HashMap<String, PhpFnMeta>,
    classes: HashMap<String, PhpClassMeta>,
    constants: HashMap<String, String>,
}

static PHP_CATALOG: OnceLock<PhpCatalog> = OnceLock::new();

fn php_catalog() -> &'static PhpCatalog {
    PHP_CATALOG.get_or_init(PhpCatalog::load)
}

impl PhpCatalog {
    fn load() -> Self {
        if let Some(cat) = Self::load_via_php_cli() {
            return cat;
        }
        Self::fallback()
    }

    fn load_via_php_cli() -> Option<Self> {
        let script = r#"
$fmtVal = function($v): string {
    if (is_int($v) || is_float($v)) return (string)$v;
    if (is_bool($v)) return $v ? "true" : "false";
    if ($v === null) return "null";
    if (is_string($v)) {
        $s = str_replace(["\\", "\n", "\r", "\t", "\""], ["\\\\", "\\n", "\\r", "\\t", "\\\""], $v);
        if (strlen($s) > 60) $s = substr($s, 0, 57) . "...";
        return "\"" . $s . "\"";
    }
    if (is_array($v)) return "array(" . count($v) . ")";
    return gettype($v);
};
$fns = [];
foreach (get_defined_functions()["internal"] as $fn) {
    try {
        $rf = new ReflectionFunction($fn);
        $ps = [];
        foreach ($rf->getParameters() as $p) {
            $t = $p->getType();
            $ts = $t instanceof ReflectionNamedType ? $t->getName() : ($t ? (string)$t : "mixed");
            if ($t instanceof ReflectionNamedType && $t->allowsNull() && $ts !== "mixed" && $ts !== "null") $ts = "?".$ts;
            $ps[] = $ts." ".($p->isVariadic() ? "...$" : "$").$p->getName();
        }
        $rt = $rf->getReturnType();
        $rts = $rt instanceof ReflectionNamedType ? $rt->getName() : ($rt ? (string)$rt : "mixed");
        if ($rt instanceof ReflectionNamedType && $rt->allowsNull() && $rts !== "mixed" && $rts !== "null") $rts = "?".$rts;
        $fns[strtolower($fn)] = ["n" => $fn, "s" => "function $fn(".implode(", ", $ps)."): $rts", "r" => $rts];
    } catch (Throwable $e) {}
}
$classes = [];
foreach (array_merge(get_declared_classes(), get_declared_interfaces()) as $cn) {
    try {
        $rc = new ReflectionClass($cn);
        if (!$rc->isInternal()) continue;
        $ms = [];
        $mnames = [];
        foreach ($rc->getMethods(ReflectionMethod::IS_PUBLIC) as $m) {
            $ps = [];
            foreach ($m->getParameters() as $p) {
                $t = $p->getType();
                $ts = $t instanceof ReflectionNamedType ? $t->getName() : ($t ? (string)$t : "mixed");
                if ($t instanceof ReflectionNamedType && $t->allowsNull() && $ts !== "mixed" && $ts !== "null") $ts = "?".$ts;
                $ps[] = $ts." ".($p->isVariadic() ? "...$" : "$").$p->getName();
            }
            $rt = $m->getReturnType() ?? (method_exists($m, "getTentativeReturnType") ? $m->getTentativeReturnType() : null);
            $rts = $rt instanceof ReflectionNamedType ? $rt->getName() : ($rt ? (string)$rt : "mixed");
            if ($rt instanceof ReflectionNamedType && $rt->allowsNull() && $rts !== "mixed" && $rts !== "null") $rts = "?".$rts;
            $mn = $m->getName();
            $ms[strtolower($mn)] = ["n" => $mn, "s" => "$cn::$mn(".implode(", ", $ps)."): $rts", "r" => $rts];
            if (count($mnames) < 8) $mnames[] = $mn;
        }
        $cv = [];
        foreach ($rc->getConstants() as $ck => $cval) {
            $cv[$ck] = $fmtVal($cval);
        }
        $kind = $rc->isInterface() ? "interface" : "class";
        $ext = $rc->getParentClass() ? " extends ".$rc->getParentClass()->getName() : "";
        $sum = "$kind $cn$ext (PHP built-in)";
        if (count($mnames) > 0) $sum .= ": ".implode(", ", $mnames).(count($ms) > 8 ? ", ..." : "");
        $classes[strtolower($cn)] = [
            "n" => $cn,
            "k" => $kind,
            "d" => $sum,
            "m" => $ms,
            "c" => $cv,
        ];
    } catch (Throwable $e) {}
}
$gconsts = [];
foreach (get_defined_constants(true) as $cat => $arr) {
    if ($cat === "user") continue;
    foreach ($arr as $ck => $cval) {
        $gconsts[$ck] = $fmtVal($cval);
    }
}
echo json_encode(["fns" => $fns, "classes" => $classes, "consts" => $gconsts]);
"#;
        let out = Command::new("php").arg("-r").arg(script).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8(out.stdout).ok()?;
        let root = parse_json(&text).ok()?;

        let mut functions = HashMap::new();
        if let Some(Json::Obj(entries)) = root.get("fns") {
            for (k, v) in entries {
                let name = v.get("n").and_then(|x| x.as_str()).unwrap_or(k).to_string();
                let sig = v.get("s").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let ret = v.get("r").and_then(|x| x.as_str()).unwrap_or("mixed").to_string();
                functions.insert(k.clone(), PhpFnMeta { name, sig, ret });
            }
        }

        let mut classes = HashMap::new();
        if let Some(Json::Obj(entries)) = root.get("classes") {
            for (k, v) in entries {
                let name = v.get("n").and_then(|x| x.as_str()).unwrap_or(k).to_string();
                let kind = match v.get("k").and_then(|x| x.as_str()) {
                    Some("interface") => "interface",
                    _ => "class",
                };
                let summary = v.get("d").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let mut methods = HashMap::new();
                if let Some(Json::Obj(ms)) = v.get("m") {
                    for (mk, mv) in ms {
                        let mn = mv.get("n").and_then(|x| x.as_str()).unwrap_or(mk).to_string();
                        let msig = mv.get("s").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let mret = mv.get("r").and_then(|x| x.as_str()).unwrap_or("mixed").to_string();
                        methods.insert(mk.clone(), PhpFnMeta { name: mn, sig: msig, ret: mret });
                    }
                }
                let mut consts = HashMap::new();
                if let Some(Json::Obj(cs)) = v.get("c") {
                    for (ck, cv) in cs {
                        if let Some(val_str) = cv.as_str() {
                            consts.insert(ck.clone(), val_str.to_string());
                        }
                    }
                }
                classes.insert(
                    k.clone(),
                    PhpClassMeta {
                        name,
                        kind,
                        summary,
                        methods,
                        consts,
                    },
                );
            }
        }

        let mut constants = HashMap::new();
        if let Some(Json::Obj(entries)) = root.get("consts") {
            for (ck, cv) in entries {
                if let Some(val_str) = cv.as_str() {
                    constants.insert(ck.clone(), val_str.to_string());
                }
            }
        }
        if functions.is_empty() && classes.is_empty() {
            return None;
        }
        Some(Self {
            functions,
            classes,
            constants,
        })
    }

    fn fallback() -> Self {
        let mut functions = HashMap::new();
        for (n, s, r) in [
            ("strlen", "function strlen(string $string): int", "int"),
            ("substr", "function substr(string $string, int $offset, ?int $length = null): string", "string"),
            ("trim", "function trim(string $string): string", "string"),
            ("json_encode", "function json_encode(mixed $value): string|false", "string|false"),
            ("json_decode", "function json_decode(string $json, ?bool $associative = null): mixed", "mixed"),
            ("count", "function count(Countable|array $value): int", "int"),
        ] {
            functions.insert(
                n.to_string(),
                PhpFnMeta {
                    name: n.to_string(),
                    sig: s.to_string(),
                    ret: r.to_string(),
                },
            );
        }
        let mut classes = HashMap::new();
        for (n, k, d) in [
            ("PDO", "class", "class PDO (PHP built-in)"),
            ("PDOStatement", "class", "class PDOStatement (PHP built-in)"),
            ("Exception", "class", "class Exception implements Throwable (PHP built-in)"),
            ("DateTimeImmutable", "class", "class DateTimeImmutable (PHP built-in)"),
        ] {
            classes.insert(
                n.to_lowercase(),
                PhpClassMeta {
                    name: n.to_string(),
                    kind: k,
                    summary: d.to_string(),
                    methods: HashMap::new(),
                    consts: HashMap::new(),
                },
            );
        }
        let mut constants = HashMap::new();
        constants.insert("PHP_EOL".to_string(), "\"\\n\"".to_string());
        Self {
            functions,
            classes,
            constants,
        }
    }
}

impl SymbolIndex {
    pub fn build(source: &str) -> Self {
        let mut idx = Self {
            defs: Vec::new(),
            occs: Vec::new(),
            imports: HashMap::new(),
        };
        let Ok(tokens) = Lexer::new(source).tokenize() else {
            return idx;
        };
        let Ok(program) = Parser::new(tokens).parse_program() else {
            return idx;
        };

        let mut global_syms: HashMap<String, usize> = HashMap::new();
        let mut class_members: HashMap<(String, String), (usize, Option<String>)> = HashMap::new();
        let mut fn_returns: HashMap<String, String> = HashMap::new();
        let mut scope_vars: Vec<HashMap<String, usize>> = vec![HashMap::new()];
        let mut var_types: Vec<HashMap<String, String>> = vec![HashMap::new()];

        // Pass 1: register top-level declarations, imports, and class fields/methods
        for item in &program.items {
            match item {
                Item::Import { names, path, span } => {
                    for nm in names {
                        let sp = find_ident_span_after(source, *span, nm);
                        let id = idx.add_def(
                            nm,
                            "import",
                            format!("import {{ {} }} from \"{}\"", nm, path),
                            sp,
                            nm.len(),
                        );
                        global_syms.insert(nm.clone(), id);
                        idx.imports.insert(id, path.clone());
                    }
                }
                Item::TypeAlias(ta) => {
                    let detail = format!("type {} = {}", ta.name, format_type_expr(&ta.ty));
                    let name_span = find_ident_span_after(source, ta.span, &ta.name);
                    let id = idx.add_def(&ta.name, "type", detail, name_span, ta.name.len());
                    global_syms.insert(ta.name.clone(), id);
                }
                Item::Interface(iface) => {
                    let detail = format!("interface {}", iface.name);
                    let name_span = find_ident_span_after(source, iface.span, &iface.name);
                    let id =
                        idx.add_def(&iface.name, "interface", detail, name_span, iface.name.len());
                    global_syms.insert(iface.name.clone(), id);
                    for m in &iface.methods {
                        let ps: Vec<String> = m
                            .params
                            .iter()
                            .map(|p| format!("{} ${}", format_type_expr(&p.ty), p.name))
                            .collect();
                        let ret_name = extract_named_type(&m.return_type);
                        let detail = format!(
                            "{}::{}({}): {}",
                            iface.name,
                            m.name,
                            ps.join(", "),
                            format_type_expr(&m.return_type)
                        );
                        let m_span = find_ident_span_after(source, m.span, &m.name);
                        let mid = idx.add_def(&m.name, "method", detail, m_span, m.name.len());
                        class_members.insert((iface.name.clone(), m.name.clone()), (mid, ret_name));
                    }
                }
                Item::Enum(en) => {
                    let detail = match &en.backing_type {
                        Some(b) => format!("enum {}: {}", en.name, b),
                        None => format!("enum {}", en.name),
                    };
                    let name_span = find_ident_span_after(source, en.span, &en.name);
                    let id = idx.add_def(&en.name, "enum", detail, name_span, en.name.len());
                    global_syms.insert(en.name.clone(), id);
                    for c in &en.cases {
                        let c_span = find_ident_span_after(source, c.span, &c.name);
                        let detail = match &c.value {
                            Some(v) => format!("{}::{} = {}", en.name, c.name, format_expr_preview(v)),
                            None => format!("{}::{}", en.name, c.name),
                        };
                        let cid = idx.add_def(
                            &c.name,
                            "enum case",
                            detail,
                            c_span,
                            c.name.len(),
                        );
                        class_members.insert(
                            (en.name.clone(), c.name.clone()),
                            (cid, Some(en.name.clone())),
                        );
                    }
                }
                Item::Class(cls) => {
                    let tps = if cls.type_params.is_empty() {
                        String::new()
                    } else {
                        format!("<{}>", cls.type_params.join(", "))
                    };
                    let detail = format!("class {}{}", cls.name, tps);
                    let name_span = find_ident_span_after(source, cls.span, &cls.name);
                    let id = idx.add_def(&cls.name, "class", detail, name_span, cls.name.len());
                    global_syms.insert(cls.name.clone(), id);
                    for f in &cls.fields {
                        let f_span = find_ident_span_after(source, f.span, &f.name);
                        let fid = idx.add_def(
                            &f.name,
                            "property",
                            format!("{}: {}", f.name, format_type_expr(&f.ty)),
                            f_span,
                            f.name.len(),
                        );
                        class_members.insert(
                            (cls.name.clone(), f.name.clone()),
                            (fid, extract_named_type(&f.ty)),
                        );
                    }
                    for m in &cls.methods {
                        let ps: Vec<String> = m
                            .params
                            .iter()
                            .map(|p| format!("{} ${}", format_type_expr(&p.ty), p.name))
                            .collect();
                        let ret_name = extract_named_type(&m.return_type);
                        let detail = format!(
                            "{}::{}({}): {}",
                            cls.name,
                            m.name,
                            ps.join(", "),
                            format_type_expr(&m.return_type)
                        );
                        let m_span = find_ident_span_after(source, m.span, &m.name);
                        let mid = idx.add_def(&m.name, "method", detail, m_span, m.name.len());
                        class_members.insert((cls.name.clone(), m.name.clone()), (mid, ret_name));
                    }
                }
                Item::Function(f) => {
                    let ps: Vec<String> = f
                        .params
                        .iter()
                        .map(|p| format!("{} ${}", format_type_expr(&p.ty), p.name))
                        .collect();
                    let detail = format!(
                        "function {}({}): {}",
                        f.name,
                        ps.join(", "),
                        format_type_expr(&f.return_type)
                    );
                    let name_span = find_ident_span_after(source, f.span, &f.name);
                    let id = idx.add_def(&f.name, "function", detail, name_span, f.name.len());
                    global_syms.insert(f.name.clone(), id);
                    if let Some(rt) = extract_named_type(&f.return_type) {
                        fn_returns.insert(f.name.clone(), rt);
                    }
                }
                _ => {}
            }
        }

        // Pass 2: index bodies & occurrences
        for item in &program.items {
            match item {
                Item::TypeAlias(ta) => {
                    index_type_expr(&ta.ty, source, &mut global_syms, &mut idx);
                }
                Item::Interface(iface) => {
                    for ext in &iface.extends {
                        if let Some(id) = resolve_class_or_type(ext, &mut global_syms, &mut idx) {
                            let clean = ext.trim_start_matches('\\');
                            let sp = find_ident_span_after(source, iface.span, clean);
                            idx.add_occ(sp, clean.len(), id);
                        }
                    }
                    for m in &iface.methods {
                        for p in &m.params {
                            index_type_expr(&p.ty, source, &mut global_syms, &mut idx);
                        }
                        index_type_expr(&m.return_type, source, &mut global_syms, &mut idx);
                    }
                }
                Item::Class(cls) => {
                    if let Some(ext) = &cls.extends {
                        if let Some(id) = resolve_class_or_type(ext, &mut global_syms, &mut idx) {
                            let clean = ext.trim_start_matches('\\');
                            let sp = find_ident_span_after(source, cls.span, clean);
                            idx.add_occ(sp, clean.len(), id);
                        }
                    }
                    for imp in &cls.implements {
                        if let Some(id) = resolve_class_or_type(imp, &mut global_syms, &mut idx) {
                            let clean = imp.trim_start_matches('\\');
                            let sp = find_ident_span_after(source, cls.span, clean);
                            idx.add_occ(sp, clean.len(), id);
                        }
                    }
                    for f in &cls.fields {
                        index_type_expr(&f.ty, source, &mut global_syms, &mut idx);
                    }
                    if let Some(ctor) = &cls.constructor {
                        scope_vars.push(HashMap::new());
                        var_types.push(HashMap::new());
                        var_types
                            .last_mut()
                            .unwrap()
                            .insert("this".to_string(), cls.name.clone());
                        for p in &ctor.params {
                            index_type_expr(&p.ty, source, &mut global_syms, &mut idx);
                            let pid = idx.add_def(
                                &format!("${}", p.name),
                                "parameter",
                                format!("{} ${}", format_type_expr(&p.ty), p.name),
                                p.span,
                                p.name.len() + 1,
                            );
                            scope_vars.last_mut().unwrap().insert(p.name.clone(), pid);
                            if let Some(tn) = extract_named_type(&p.ty) {
                                var_types.last_mut().unwrap().insert(p.name.clone(), tn);
                            }
                        }
                        let mut ctx = IndexCtx {
                            source,
                            current_class: Some(&cls.name),
                            globals: &mut global_syms,
                            class_members: &class_members,
                            fn_returns: &fn_returns,
                            scopes: &mut scope_vars,
                            var_types: &mut var_types,
                            idx: &mut idx,
                        };
                        index_stmts(&ctor.body, &mut ctx);
                        scope_vars.pop();
                        var_types.pop();
                    }
                    for m in &cls.methods {
                        scope_vars.push(HashMap::new());
                        var_types.push(HashMap::new());
                        var_types
                            .last_mut()
                            .unwrap()
                            .insert("this".to_string(), cls.name.clone());
                        for p in &m.params {
                            index_type_expr(&p.ty, source, &mut global_syms, &mut idx);
                            let pid = idx.add_def(
                                &format!("${}", p.name),
                                "parameter",
                                format!("{} ${}", format_type_expr(&p.ty), p.name),
                                p.span,
                                p.name.len() + 1,
                            );
                            scope_vars.last_mut().unwrap().insert(p.name.clone(), pid);
                            if let Some(tn) = extract_named_type(&p.ty) {
                                var_types.last_mut().unwrap().insert(p.name.clone(), tn);
                            }
                        }
                        index_type_expr(&m.return_type, source, &mut global_syms, &mut idx);
                        let mut ctx = IndexCtx {
                            source,
                            current_class: Some(&cls.name),
                            globals: &mut global_syms,
                            class_members: &class_members,
                            fn_returns: &fn_returns,
                            scopes: &mut scope_vars,
                            var_types: &mut var_types,
                            idx: &mut idx,
                        };
                        index_stmts(&m.body, &mut ctx);
                        scope_vars.pop();
                        var_types.pop();
                    }
                }
                Item::Function(f) => {
                    scope_vars.push(HashMap::new());
                    var_types.push(HashMap::new());
                    for p in &f.params {
                        index_type_expr(&p.ty, source, &mut global_syms, &mut idx);
                        let pid = idx.add_def(
                            &format!("${}", p.name),
                            "parameter",
                            format!("{} ${}", format_type_expr(&p.ty), p.name),
                            p.span,
                            p.name.len() + 1,
                        );
                        scope_vars.last_mut().unwrap().insert(p.name.clone(), pid);
                        if let Some(tn) = extract_named_type(&p.ty) {
                            var_types.last_mut().unwrap().insert(p.name.clone(), tn);
                        }
                    }
                    index_type_expr(&f.return_type, source, &mut global_syms, &mut idx);
                    let mut ctx = IndexCtx {
                        source,
                        current_class: None,
                        globals: &mut global_syms,
                        class_members: &class_members,
                        fn_returns: &fn_returns,
                        scopes: &mut scope_vars,
                        var_types: &mut var_types,
                        idx: &mut idx,
                    };
                    index_stmts(&f.body, &mut ctx);
                    scope_vars.pop();
                    var_types.pop();
                }
                Item::Stmt(s) => {
                    let mut ctx = IndexCtx {
                        source,
                        current_class: None,
                        globals: &mut global_syms,
                        class_members: &class_members,
                        fn_returns: &fn_returns,
                        scopes: &mut scope_vars,
                        var_types: &mut var_types,
                        idx: &mut idx,
                    };
                    index_stmt(s, &mut ctx);
                }
                _ => {}
            }
        }

        idx
    }

    fn add_def(
        &mut self,
        name: &str,
        kind: &'static str,
        detail: String,
        span: Span,
        len: usize,
    ) -> usize {
        let id = self.defs.len();
        self.defs.push(SymbolDef {
            id,
            name: name.to_string(),
            kind,
            detail,
            span,
            len,
        });
        self.occs.push(SymbolOcc {
            span,
            len,
            def_id: id,
        });
        id
    }

    fn add_builtin_def(&mut self, name: &str, kind: &'static str, detail: String) -> usize {
        let id = self.defs.len();
        self.defs.push(SymbolDef {
            id,
            name: name.to_string(),
            kind,
            detail,
            span: Span::new(0, 0),
            len: name.len(),
        });
        id
    }

    fn add_occ(&mut self, span: Span, len: usize, def_id: usize) {
        self.occs.push(SymbolOcc { span, len, def_id });
    }

    /// Find symbol definition at 0-based LSP `(line, character)`.
    pub fn find_at(&self, lsp_line: usize, lsp_col: usize) -> Option<&SymbolDef> {
        let target_line = lsp_line + 1;
        let target_col = lsp_col + 1;
        for occ in &self.occs {
            if occ.span.line == target_line
                && target_col >= occ.span.col
                && target_col < occ.span.col + occ.len.max(1)
            {
                return self.defs.get(occ.def_id);
            }
        }
        None
    }

    /// Return all occurrences (including declaration) for a `def_id`.
    pub fn references_for(&self, def_id: usize) -> Vec<&SymbolOcc> {
        self.occs.iter().filter(|o| o.def_id == def_id).collect()
    }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn find_ident_span_after(source: &str, approx: Span, name: &str) -> Span {
    if let Some(line_str) = source.lines().nth(approx.line.saturating_sub(1)) {
        let mut search_from = approx.col.saturating_sub(1).min(line_str.len());
        let bytes = line_str.as_bytes();
        while let Some(rel) = line_str[search_from..].find(name) {
            let pos = search_from + rel;
            let before_ok = pos == 0 || !is_ident_char(bytes[pos - 1]);
            let after_pos = pos + name.len();
            let after_ok = after_pos >= bytes.len() || !is_ident_char(bytes[after_pos]);
            if before_ok && after_ok {
                return Span::new(approx.line, pos + 1);
            }
            search_from = pos + 1;
        }
    }
    approx
}

fn lookup_var_scope(scopes: &[HashMap<String, usize>], name: &str) -> Option<usize> {
    for sc in scopes.iter().rev() {
        if let Some(&id) = sc.get(name) {
            return Some(id);
        }
    }
    None
}

fn is_primitive_type_name(name: &str) -> bool {
    matches!(
        name,
        "int"
            | "float"
            | "string"
            | "bool"
            | "void"
            | "mixed"
            | "never"
            | "null"
            | "any"
            | "array"
            | "dict"
            | "self"
            | "static"
            | "parent"
            | "callable"
            | "iterable"
            | "object"
            | "true"
            | "false"
    )
}

struct IndexCtx<'a> {
    source: &'a str,
    current_class: Option<&'a str>,
    globals: &'a mut HashMap<String, usize>,
    class_members: &'a HashMap<(String, String), (usize, Option<String>)>,
    fn_returns: &'a HashMap<String, String>,
    scopes: &'a mut Vec<HashMap<String, usize>>,
    var_types: &'a mut Vec<HashMap<String, String>>,
    idx: &'a mut SymbolIndex,
}

fn lookup_var_type(var_types: &[HashMap<String, String>], name: &str) -> Option<String> {
    for vt in var_types.iter().rev() {
        if let Some(t) = vt.get(name) {
            return Some(t.clone());
        }
    }
    None
}

fn extract_named_type(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Named(n, _, _) => {
            let clean = n.trim_start_matches('\\');
            if !clean.is_empty() && !is_primitive_type_name(clean) {
                Some(clean.to_string())
            } else {
                None
            }
        }
        TypeExpr::Optional(inner, _) => extract_named_type(inner),
        _ => None,
    }
}

fn resolve_class_or_type(
    raw_name: &str,
    globals: &mut HashMap<String, usize>,
    idx: &mut SymbolIndex,
) -> Option<usize> {
    let clean = raw_name.trim_start_matches('\\');
    if clean.is_empty() || is_primitive_type_name(clean) {
        return None;
    }
    if let Some(&id) = globals.get(clean) {
        return Some(id);
    }
    if let Some(meta) = php_catalog().classes.get(&clean.to_lowercase()) {
        let id = idx.add_builtin_def(&meta.name, meta.kind, meta.summary.clone());
        globals.insert(clean.to_string(), id);
        return Some(id);
    }
    let looks_like_class = clean
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
        || clean.contains('\\');
    if looks_like_class {
        let id = idx.add_builtin_def(
            clean,
            "class",
            format!("class {} (PHP / external)", clean),
        );
        globals.insert(clean.to_string(), id);
        return Some(id);
    }
    None
}

fn resolve_function(
    raw_name: &str,
    globals: &mut HashMap<String, usize>,
    idx: &mut SymbolIndex,
) -> Option<usize> {
    let clean = raw_name.trim_start_matches('\\');
    if clean.is_empty() {
        return None;
    }
    if let Some(&id) = globals.get(clean) {
        return Some(id);
    }
    if let Some(meta) = php_catalog().functions.get(&clean.to_lowercase()) {
        let id = idx.add_builtin_def(&meta.name, "function", meta.sig.clone());
        globals.insert(clean.to_string(), id);
        return Some(id);
    }
    if let Some(val) = php_catalog().constants.get(clean) {
        let id = idx.add_builtin_def(
            clean,
            "constant",
            format!("const {} = {} (PHP built-in)", clean, val),
        );
        globals.insert(clean.to_string(), id);
        return Some(id);
    }
    None
}

fn resolve_member_symbol(
    class_name: &str,
    member: &str,
    globals: &mut HashMap<String, usize>,
    class_members: &HashMap<(String, String), (usize, Option<String>)>,
    idx: &mut SymbolIndex,
) -> Option<usize> {
    let clean_cls = class_name.trim_start_matches('\\');
    if let Some(&(id, _)) = class_members.get(&(clean_cls.to_string(), member.to_string())) {
        return Some(id);
    }
    let key = format!("{}::{}", clean_cls, member);
    if let Some(&id) = globals.get(&key) {
        return Some(id);
    }
    if let Some(cls_meta) = php_catalog().classes.get(&clean_cls.to_lowercase()) {
        if let Some(m_meta) = cls_meta.methods.get(&member.to_lowercase()) {
            let id = idx.add_builtin_def(&m_meta.name, "method", m_meta.sig.clone());
            globals.insert(key, id);
            return Some(id);
        }
        if let Some(val) = cls_meta.consts.get(member) {
            let id = idx.add_builtin_def(
                member,
                "constant",
                format!("const {}::{} = {} (PHP built-in)", cls_meta.name, member, val),
            );
            globals.insert(key, id);
            return Some(id);
        }
    }
    None
}

fn extract_class_from_php_type(ty: &str) -> Option<String> {
    for part in ty.split('|') {
        let p = part.trim().trim_start_matches('?');
        if !p.is_empty() && !is_primitive_type_name(p) {
            return Some(p.to_string());
        }
    }
    None
}

fn infer_expr_type_name(
    expr: &Expr,
    current_class: Option<&str>,
    var_types: &[HashMap<String, String>],
    fn_returns: &HashMap<String, String>,
    class_members: &HashMap<(String, String), (usize, Option<String>)>,
) -> Option<String> {
    match expr {
        Expr::Var(name, _) => {
            if name == "this" {
                current_class.map(|s| s.to_string())
            } else {
                lookup_var_type(var_types, name)
            }
        }
        Expr::New { class_name, .. } => {
            let clean = class_name.trim_start_matches('\\');
            if clean.is_empty() {
                None
            } else {
                Some(clean.to_string())
            }
        }
        Expr::Call { callee, .. } => match callee.as_ref() {
            Expr::Ident(fn_name, _) => {
                let clean = fn_name.trim_start_matches('\\');
                if let Some(rt) = fn_returns.get(clean) {
                    return Some(rt.clone());
                }
                if let Some(meta) = php_catalog().functions.get(&clean.to_lowercase()) {
                    return extract_class_from_php_type(&meta.ret);
                }
                None
            }
            Expr::Member {
                object,
                property: member,
                ..
            } => {
                let obj_ty = infer_expr_type_name(
                    object,
                    current_class,
                    var_types,
                    fn_returns,
                    class_members,
                )?;
                if let Some((_, Some(rt))) = class_members.get(&(obj_ty.clone(), member.clone())) {
                    return Some(rt.clone());
                }
                if let Some(cls_meta) = php_catalog().classes.get(&obj_ty.to_lowercase()) {
                    if let Some(m_meta) = cls_meta.methods.get(&member.to_lowercase()) {
                        return extract_class_from_php_type(&m_meta.ret);
                    }
                }
                None
            }
            _ => None,
        },
        Expr::StaticAccess {
            class_name, member, ..
        } => {
            let clean_cls = class_name.trim_start_matches('\\');
            if let Some((_, Some(rt))) = class_members.get(&(clean_cls.to_string(), member.clone())) {
                return Some(rt.clone());
            }
            if let Some(cls_meta) = php_catalog().classes.get(&clean_cls.to_lowercase()) {
                if let Some(m_meta) = cls_meta.methods.get(&member.to_lowercase()) {
                    return extract_class_from_php_type(&m_meta.ret);
                }
            }
            None
        }
        _ => None,
    }
}

fn index_type_expr(
    ty: &TypeExpr,
    source: &str,
    globals: &mut HashMap<String, usize>,
    idx: &mut SymbolIndex,
) {
    match ty {
        TypeExpr::Named(name, args, span) => {
            if let Some(id) = resolve_class_or_type(name, globals, idx) {
                let clean = name.trim_start_matches('\\');
                let sp = if name.starts_with('\\') {
                    find_ident_span_after(source, *span, clean)
                } else {
                    *span
                };
                idx.add_occ(sp, clean.len(), id);
            }
            for a in args {
                index_type_expr(a, source, globals, idx);
            }
        }
        TypeExpr::Array(inner, _) | TypeExpr::Optional(inner, _) => {
            index_type_expr(inner, source, globals, idx);
        }
        TypeExpr::Union(items, _) => {
            for t in items {
                index_type_expr(t, source, globals, idx);
            }
        }
        TypeExpr::Shape(fields, _) => {
            for f in fields {
                index_type_expr(&f.ty, source, globals, idx);
            }
        }
        TypeExpr::Function { params, ret, .. } => {
            for p in params {
                index_type_expr(p, source, globals, idx);
            }
            index_type_expr(ret, source, globals, idx);
        }
    }
}

fn index_stmts(stmts: &[Stmt], ctx: &mut IndexCtx<'_>) {
    for s in stmts {
        index_stmt(s, ctx);
    }
}

fn index_stmt(stmt: &Stmt, ctx: &mut IndexCtx<'_>) {
    match stmt {
        Stmt::VarDecl {
            name,
            ty,
            init,
            span,
        } => {
            if let Some(t) = ty {
                index_type_expr(t, ctx.source, ctx.globals, ctx.idx);
            }
            index_expr(init, ctx);
            let inferred_cls = ty
                .as_ref()
                .and_then(extract_named_type)
                .or_else(|| {
                    infer_expr_type_name(
                        init,
                        ctx.current_class,
                        ctx.var_types,
                        ctx.fn_returns,
                        ctx.class_members,
                    )
                });
            let ty_str = ty
                .as_ref()
                .map(format_type_expr)
                .or_else(|| inferred_cls.clone())
                .unwrap_or_else(|| "inferred".to_string());
            let id = ctx.idx.add_def(
                &format!("${}", name),
                "variable",
                format!("{}: {}", format!("${}", name), ty_str),
                *span,
                name.len() + 1,
            );
            ctx.scopes.last_mut().unwrap().insert(name.clone(), id);
            if let Some(c) = inferred_cls {
                ctx.var_types.last_mut().unwrap().insert(name.clone(), c);
            }
        }
        Stmt::ConstDecl {
            name,
            ty,
            init,
            span,
        } => {
            if let Some(t) = ty {
                index_type_expr(t, ctx.source, ctx.globals, ctx.idx);
            }
            index_expr(init, ctx);
            let val_preview = format_expr_preview(init);
            let detail = match ty.as_ref() {
                Some(t) => format!("const {}: {} = {}", name, format_type_expr(t), val_preview),
                None => format!("const {} = {}", name, val_preview),
            };
            let name_span = find_ident_span_after(ctx.source, *span, name);
            let id = ctx.idx.add_def(name, "constant", detail, name_span, name.len());
            ctx.scopes.last_mut().unwrap().insert(name.clone(), id);
        }
        Stmt::Assign { target, value, .. } => {
            index_expr(value, ctx);
            let inferred_cls = infer_expr_type_name(
                value,
                ctx.current_class,
                ctx.var_types,
                ctx.fn_returns,
                ctx.class_members,
            );
            if let Expr::Var(vname, vspan) = target {
                if let Some(id) = lookup_var_scope(ctx.scopes, vname) {
                    ctx.idx.add_occ(*vspan, vname.len() + 1, id);
                } else {
                    let ty_str = inferred_cls
                        .clone()
                        .unwrap_or_else(|| "inferred".to_string());
                    let id = ctx.idx.add_def(
                        &format!("${}", vname),
                        "variable",
                        format!("${}: {}", vname, ty_str),
                        *vspan,
                        vname.len() + 1,
                    );
                    ctx.scopes.last_mut().unwrap().insert(vname.clone(), id);
                }
                if let Some(c) = inferred_cls {
                    ctx.var_types.last_mut().unwrap().insert(vname.clone(), c);
                }
            } else {
                index_expr(target, ctx);
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                index_expr(v, ctx);
            }
        }
        Stmt::Echo { value, .. } => index_expr(value, ctx),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            index_expr(cond, ctx);
            index_stmts(then_branch, ctx);
            if let Some(eb) = else_branch {
                index_stmts(eb, ctx);
            }
        }
        Stmt::While { cond, body, .. } => {
            index_expr(cond, ctx);
            index_stmts(body, ctx);
        }
        Stmt::ForEach {
            iterable,
            key_var,
            val_var,
            body,
            span,
        } => {
            index_expr(iterable, ctx);
            ctx.scopes.push(HashMap::new());
            ctx.var_types.push(HashMap::new());
            if let Some(kv) = key_var {
                let kid = ctx.idx.add_def(
                    &format!("${}", kv),
                    "variable",
                    format!("${} (foreach key)", kv),
                    *span,
                    kv.len() + 1,
                );
                ctx.scopes.last_mut().unwrap().insert(kv.clone(), kid);
            }
            let vid = ctx.idx.add_def(
                &format!("${}", val_var),
                "variable",
                format!("${} (foreach value)", val_var),
                *span,
                val_var.len() + 1,
            );
            ctx.scopes.last_mut().unwrap().insert(val_var.clone(), vid);
            index_stmts(body, ctx);
            ctx.scopes.pop();
            ctx.var_types.pop();
        }
        Stmt::ExprStmt { expr, .. } => index_expr(expr, ctx),
    }
}

fn index_expr(expr: &Expr, ctx: &mut IndexCtx<'_>) {
    match expr {
        Expr::Var(name, span) => {
            if let Some(id) = lookup_var_scope(ctx.scopes, name) {
                ctx.idx.add_occ(*span, name.len() + 1, id);
            }
        }
        Expr::Ident(name, span) => {
            let clean = name.trim_start_matches('\\');
            if let Some(id) = resolve_function(clean, ctx.globals, ctx.idx) {
                let sp = if name.starts_with('\\') {
                    find_ident_span_after(ctx.source, *span, clean)
                } else {
                    *span
                };
                ctx.idx.add_occ(sp, clean.len(), id);
            } else if let Some(id) = lookup_var_scope(ctx.scopes, name) {
                ctx.idx.add_occ(*span, name.len(), id);
            }
        }
        Expr::New {
            class_name,
            type_args,
            args,
            span,
        } => {
            if let Some(id) = resolve_class_or_type(class_name, ctx.globals, ctx.idx) {
                let clean = class_name.trim_start_matches('\\');
                let cls_span = find_ident_span_after(ctx.source, *span, clean);
                ctx.idx.add_occ(cls_span, clean.len(), id);
            }
            for ta in type_args {
                index_type_expr(ta, ctx.source, ctx.globals, ctx.idx);
            }
            for a in args {
                index_expr(a, ctx);
            }
        }
        Expr::StaticAccess {
            class_name,
            member,
            span,
        } => {
            if let Some(id) = resolve_class_or_type(class_name, ctx.globals, ctx.idx) {
                let clean = class_name.trim_start_matches('\\');
                let cls_span = find_ident_span_after(ctx.source, *span, clean);
                ctx.idx.add_occ(cls_span, clean.len(), id);
                if let Some(mid) =
                    resolve_member_symbol(clean, member, ctx.globals, ctx.class_members, ctx.idx)
                {
                    let m_span = find_ident_span_after(ctx.source, *span, member);
                    ctx.idx.add_occ(m_span, member.len(), mid);
                }
            }
        }
        Expr::InstanceOf {
            expr,
            class_name,
            span,
        } => {
            index_expr(expr, ctx);
            if let Some(id) = resolve_class_or_type(class_name, ctx.globals, ctx.idx) {
                let clean = class_name.trim_start_matches('\\');
                let cls_span = find_ident_span_after(ctx.source, *span, clean);
                ctx.idx.add_occ(cls_span, clean.len(), id);
            }
        }
        Expr::ArrayLit(items, _) => {
            for it in items {
                index_expr(it, ctx);
            }
        }
        Expr::AssocArrayLit(entries, _) => {
            for (_, v) in entries {
                index_expr(v, ctx);
            }
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } => {
            index_expr(expr, ctx);
        }
        Expr::Binary { left, right, .. } => {
            index_expr(left, ctx);
            index_expr(right, ctx);
        }
        Expr::Call {
            callee,
            type_args,
            args,
            ..
        } => {
            index_expr(callee, ctx);
            for ta in type_args {
                index_type_expr(ta, ctx.source, ctx.globals, ctx.idx);
            }
            for a in args {
                index_expr(a, ctx);
            }
        }
        Expr::Member {
            object,
            property: member,
            span,
        } => {
            index_expr(object, ctx);
            if let Some(obj_ty) = infer_expr_type_name(
                object,
                ctx.current_class,
                ctx.var_types,
                ctx.fn_returns,
                ctx.class_members,
            ) {
                if let Some(mid) = resolve_member_symbol(
                    &obj_ty,
                    member,
                    ctx.globals,
                    ctx.class_members,
                    ctx.idx,
                ) {
                    let m_span = find_ident_span_after(ctx.source, *span, member);
                    ctx.idx.add_occ(m_span, member.len(), mid);
                }
            }
        }
        Expr::Index { object, index, .. } => {
            index_expr(object, ctx);
            index_expr(index, ctx);
        }
        Expr::ArrowFn {
            params,
            return_type,
            body,
            ..
        } => {
            ctx.scopes.push(HashMap::new());
            ctx.var_types.push(HashMap::new());
            for p in params {
                index_type_expr(&p.ty, ctx.source, ctx.globals, ctx.idx);
                let pid = ctx.idx.add_def(
                    &format!("${}", p.name),
                    "parameter",
                    format!("{} ${}", format_type_expr(&p.ty), p.name),
                    p.span,
                    p.name.len() + 1,
                );
                ctx.scopes.last_mut().unwrap().insert(p.name.clone(), pid);
                if let Some(tn) = extract_named_type(&p.ty) {
                    ctx.var_types
                        .last_mut()
                        .unwrap()
                        .insert(p.name.clone(), tn);
                }
            }
            if let Some(rt) = return_type {
                index_type_expr(rt, ctx.source, ctx.globals, ctx.idx);
            }
            index_expr(body, ctx);
            ctx.scopes.pop();
            ctx.var_types.pop();
        }
        Expr::Int(..) | Expr::Float(..) | Expr::String(..) | Expr::Bool(..) | Expr::Null(..) => {}
    }
}

fn format_expr_preview(expr: &Expr) -> String {
    match expr {
        Expr::Int(v, _) => v.to_string(),
        Expr::Float(v, _) => v.to_string(),
        Expr::String(s, _) => {
            let esc = s
                .replace('\\', "\\\\")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t")
                .replace('"', "\\\"");
            if esc.len() > 60 {
                format!("\"{}...\"", &esc[..57])
            } else {
                format!("\"{}\"", esc)
            }
        }
        Expr::Bool(b, _) => b.to_string(),
        Expr::Null(_) => "null".to_string(),
        Expr::Var(n, _) => format!("${}", n),
        Expr::Ident(n, _) => n.clone(),
        Expr::Unary { op, expr, .. } => {
            let sym = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            };
            format!("{}{}", sym, format_expr_preview(expr))
        }
        Expr::StaticAccess {
            class_name, member, ..
        } => format!("{}::{}", class_name, member),
        Expr::ArrayLit(items, _) => {
            let shown: Vec<String> = items.iter().take(4).map(format_expr_preview).collect();
            if items.len() > 4 {
                format!("[{}, ...]", shown.join(", "))
            } else {
                format!("[{}]", shown.join(", "))
            }
        }
        Expr::AssocArrayLit(entries, _) => {
            let shown: Vec<String> = entries
                .iter()
                .take(3)
                .map(|(k, v)| format!("\"{}\" => {}", k, format_expr_preview(v)))
                .collect();
            if entries.len() > 3 {
                format!("[{}, ...]", shown.join(", "))
            } else {
                format!("[{}]", shown.join(", "))
            }
        }
        _ => "...".to_string(),
    }
}

fn format_type_expr(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(n, args, _) => {
            if args.is_empty() {
                n.clone()
            } else {
                let ps: Vec<String> = args.iter().map(format_type_expr).collect();
                format!("{}<{}>", n, ps.join(", "))
            }
        }
        TypeExpr::Array(e, _) => format!("{}[]", format_type_expr(e)),
        TypeExpr::Optional(e, _) => format!("?{}", format_type_expr(e)),
        TypeExpr::Union(items, _) => items
            .iter()
            .map(format_type_expr)
            .collect::<Vec<_>>()
            .join("|"),
        TypeExpr::Shape(fields, _) => {
            let ps: Vec<String> = fields
                .iter()
                .map(|f| {
                    let q = if f.optional { "?" } else { "" };
                    format!("{}{}: {}", f.name, q, format_type_expr(&f.ty))
                })
                .collect();
            format!("{{ {} }}", ps.join(", "))
        }
        TypeExpr::Function { params, ret, .. } => {
            let ps: Vec<String> = params.iter().map(format_type_expr).collect();
            format!("({}) => {}", ps.join(", "), format_type_expr(ret))
        }
    }
}

// ============================================================================
// 3. LSP Stdio Server Loop
// ============================================================================

pub fn run_lsp_server() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    let mut docs: HashMap<String, (String, SymbolIndex)> = HashMap::new();

    while let Some(msg) = read_lsp_message(&mut reader) {
        let Ok(json) = parse_json(&msg) else {
            continue;
        };
        let method = json.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = json.get("id").cloned();

        match method {
            "initialize" => {
                let result = Json::Obj(vec![(
                    "capabilities".into(),
                    Json::Obj(vec![
                        ("textDocumentSync".into(), Json::Num(1.0)),
                        ("hoverProvider".into(), Json::Bool(true)),
                        ("definitionProvider".into(), Json::Bool(true)),
                        ("referencesProvider".into(), Json::Bool(true)),
                    ]),
                )]);
                if let Some(req_id) = id {
                    send_lsp_response(&mut writer, req_id, result);
                }
            }
            "shutdown" => {
                if let Some(req_id) = id {
                    send_lsp_response(&mut writer, req_id, Json::Null);
                }
            }
            "exit" => break,
            "textDocument/didOpen" => {
                if let Some(td) = json.get("params").and_then(|p| p.get("textDocument")) {
                    if let (Some(uri), Some(text)) = (
                        td.get("uri").and_then(|u| u.as_str()),
                        td.get("text").and_then(|t| t.as_str()),
                    ) {
                        let index = SymbolIndex::build(text);
                        docs.insert(uri.to_string(), (text.to_string(), index));
                        publish_diagnostics(&mut writer, uri, text);
                    }
                }
            }
            "textDocument/didChange" => {
                if let Some(params) = json.get("params") {
                    let uri = params
                        .get("textDocument")
                        .and_then(|td| td.get("uri"))
                        .and_then(|u| u.as_str());
                    let text = params
                        .get("contentChanges")
                        .and_then(|cc| match cc {
                            Json::Arr(arr) => arr.first(),
                            _ => None,
                        })
                        .and_then(|c| c.get("text"))
                        .and_then(|t| t.as_str());
                    if let (Some(u), Some(t)) = (uri, text) {
                        let index = SymbolIndex::build(t);
                        docs.insert(u.to_string(), (t.to_string(), index));
                        publish_diagnostics(&mut writer, u, t);
                    }
                }
            }
            "textDocument/hover" => {
                let res = handle_hover(&docs, &json);
                if let Some(req_id) = id {
                    send_lsp_response(&mut writer, req_id, res);
                }
            }
            "textDocument/definition" => {
                let res = handle_definition(&docs, &json);
                if let Some(req_id) = id {
                    send_lsp_response(&mut writer, req_id, res);
                }
            }
            "textDocument/references" => {
                let res = handle_references(&docs, &json);
                if let Some(req_id) = id {
                    send_lsp_response(&mut writer, req_id, res);
                }
            }
            _ => {
                if let Some(req_id) = id {
                    send_lsp_response(&mut writer, req_id, Json::Null);
                }
            }
        }
    }
}

fn extract_uri_pos(json: &Json) -> Option<(&str, usize, usize)> {
    let params = json.get("params")?;
    let uri = params.get("textDocument")?.get("uri")?.as_str()?;
    let pos = params.get("position")?;
    let line = pos.get("line")?.as_usize()?;
    let ch = pos.get("character")?.as_usize()?;
    Some((uri, line, ch))
}

fn span_to_range(span: Span, len: usize) -> Json {
    let line0 = span.line.saturating_sub(1) as f64;
    let col0 = span.col.saturating_sub(1) as f64;
    let end_col = col0 + (len.max(1) as f64);
    Json::Obj(vec![
        (
            "start".into(),
            Json::Obj(vec![("line".into(), Json::Num(line0)), ("character".into(), Json::Num(col0))]),
        ),
        (
            "end".into(),
            Json::Obj(vec![("line".into(), Json::Num(line0)), ("character".into(), Json::Num(end_col))]),
        ),
    ])
}

fn handle_hover(docs: &HashMap<String, (String, SymbolIndex)>, json: &Json) -> Json {
    let Some((uri, line, ch)) = extract_uri_pos(json) else {
        return Json::Null;
    };
    let Some((_, index)) = docs.get(uri) else {
        return Json::Null;
    };
    let Some(def) = index.find_at(line, ch) else {
        return Json::Null;
    };
    let ref_count = index.references_for(def.id).len();
    let location_note = if def.span.line == 0 {
        format!("PHP built-in / external symbol • {} reference(s) in file", ref_count)
    } else {
        format!("Defined at line {} • {} reference(s)", def.span.line, ref_count)
    };
    let md = format!(
        "```typephp\n({}) {}\n```\n{}",
        def.kind, def.detail, location_note
    );
    Json::Obj(vec![(
        "contents".into(),
        Json::Obj(vec![
            ("kind".into(), Json::Str("markdown".into())),
            ("value".into(), Json::Str(md)),
        ]),
    )])
}

fn handle_definition(docs: &HashMap<String, (String, SymbolIndex)>, json: &Json) -> Json {
    let Some((uri, line, ch)) = extract_uri_pos(json) else {
        return Json::Null;
    };
    let Some((_, index)) = docs.get(uri) else {
        return Json::Null;
    };
    let Some(def) = index.find_at(line, ch) else {
        return Json::Null;
    };
    if let Some(rel_path) = index.imports.get(&def.id) {
        if let Some(cur_path) = uri.strip_prefix("file://") {
            let base_dir = Path::new(cur_path).parent().unwrap_or_else(|| Path::new("."));
            let mut target = base_dir.join(rel_path);
            if !target.exists() && target.extension().is_none() {
                let with_ext = PathBuf::from(format!("{}.tphp", target.display()));
                if with_ext.exists() {
                    target = with_ext;
                }
            }
            if let Ok(canon) = std::fs::canonicalize(&target) {
                let target_uri = format!("file://{}", canon.display());
                let target_idx = docs
                    .get(&target_uri)
                    .map(|(_, i)| i.clone())
                    .or_else(|| {
                        std::fs::read_to_string(&canon)
                            .ok()
                            .map(|src| SymbolIndex::build(&src))
                    });
                if let Some(ti) = target_idx {
                    if let Some(target_def) = ti.defs.iter().find(|d| d.name == def.name && d.span.line > 0) {
                        return Json::Obj(vec![
                            ("uri".into(), Json::Str(target_uri)),
                            ("range".into(), span_to_range(target_def.span, target_def.len)),
                        ]);
                    }
                }
            }
        }
    }
    if def.span.line == 0 {
        let locs: Vec<Json> = index
            .references_for(def.id)
            .into_iter()
            .map(|occ| {
                Json::Obj(vec![
                    ("uri".into(), Json::Str(uri.to_string())),
                    ("range".into(), span_to_range(occ.span, occ.len)),
                ])
            })
            .collect();
        return if locs.is_empty() {
            Json::Null
        } else {
            Json::Arr(locs)
        };
    }
    let at_definition = def.span.line == line + 1
        && (ch + 1) >= def.span.col
        && (ch + 1) < def.span.col + def.len.max(1);
    if at_definition {
        let refs: Vec<Json> = index
            .references_for(def.id)
            .into_iter()
            .filter(|occ| !(occ.span.line == def.span.line && occ.span.col == def.span.col))
            .map(|occ| {
                Json::Obj(vec![
                    ("uri".into(), Json::Str(uri.to_string())),
                    ("range".into(), span_to_range(occ.span, occ.len)),
                ])
            })
            .collect();
        if !refs.is_empty() {
            return Json::Arr(refs);
        }
    }
    Json::Obj(vec![
        ("uri".into(), Json::Str(uri.to_string())),
        ("range".into(), span_to_range(def.span, def.len)),
    ])
}

fn handle_references(docs: &HashMap<String, (String, SymbolIndex)>, json: &Json) -> Json {
    let Some((uri, line, ch)) = extract_uri_pos(json) else {
        return Json::Arr(vec![]);
    };
    let Some((_, index)) = docs.get(uri) else {
        return Json::Arr(vec![]);
    };
    let Some(def) = index.find_at(line, ch) else {
        return Json::Arr(vec![]);
    };
    let locs = index
        .references_for(def.id)
        .into_iter()
        .map(|occ| {
            Json::Obj(vec![
                ("uri".into(), Json::Str(uri.to_string())),
                ("range".into(), span_to_range(occ.span, occ.len)),
            ])
        })
        .collect();
    Json::Arr(locs)
}

fn publish_diagnostics(writer: &mut impl Write, uri: &str, source: &str) {
    let mut diags = Vec::new();
    match check_with_warnings(source) {
        Ok(warnings) => {
            for w in warnings {
                diags.push(Json::Obj(vec![
                    ("range".into(), span_to_range(w.span, 1)),
                    ("severity".into(), Json::Num(2.0)), // Warning
                    ("source".into(), Json::Str("typephp".into())),
                    ("message".into(), Json::Str(w.message)),
                ]));
            }
        }
        Err(err) => {
            diags.push(Json::Obj(vec![
                ("range".into(), span_to_range(err.span, 1)),
                ("severity".into(), Json::Num(1.0)), // Error
                ("source".into(), Json::Str(err.stage.into())),
                ("message".into(), Json::Str(err.message)),
            ]));
        }
    }
    let notif = Json::Obj(vec![
        ("jsonrpc".into(), Json::Str("2.0".into())),
        ("method".into(), Json::Str("textDocument/publishDiagnostics".into())),
        (
            "params".into(),
            Json::Obj(vec![
                ("uri".into(), Json::Str(uri.to_string())),
                ("diagnostics".into(), Json::Arr(diags)),
            ]),
        ),
    ]);
    write_lsp_message(writer, &notif);
}

fn read_lsp_message(reader: &mut impl BufRead) -> Option<String> {
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
            content_length = len_str.trim().parse().unwrap_or(0);
        }
    }
    if content_length == 0 {
        return None;
    }
    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

fn send_lsp_response(writer: &mut impl Write, id: Json, result: Json) {
    let resp = Json::Obj(vec![
        ("jsonrpc".into(), Json::Str("2.0".into())),
        ("id".into(), id),
        ("result".into(), result),
    ]);
    write_lsp_message(writer, &resp);
}

fn write_lsp_message(writer: &mut impl Write, json: &Json) {
    let body = json.to_string_compact();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let _ = writer.write_all(header.as_bytes());
    let _ = writer.write_all(body.as_bytes());
    let _ = writer.flush();
}
