use std::collections::{HashMap, HashSet};

use crate::ast::*;

fn short_ident(mut idx: usize) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push((b'a' + (idx % 26) as u8) as char);
        if idx < 26 {
            break;
        }
        idx = idx / 26 - 1;
    }
    chars.into_iter().rev().collect()
}

fn short_fn_ident(idx: usize) -> String {
    format!("_{}", short_ident(idx))
}

fn is_reserved_var(name: &str) -> bool {
    matches!(
        name,
        "this"
            | "_GET"
            | "_POST"
            | "_SERVER"
            | "_COOKIE"
            | "_FILES"
            | "_ENV"
            | "_REQUEST"
            | "_SESSION"
            | "GLOBALS"
    )
}

pub fn mangle_programs(programs: &mut [Program], mangle_top_level_fns: bool) {
    let mut fn_map: HashMap<(Option<String>, String), String> = HashMap::new();
    if mangle_top_level_fns {
        let mut fn_idx = 0usize;
        for prog in programs.iter() {
            let ns = prog.items.iter().find_map(|it| match it {
                Item::Namespace(n, _) => Some(n.clone()),
                _ => None,
            });
            for item in &prog.items {
                if let Item::Function(f) = item {
                    if f.name == "main" {
                        continue;
                    }
                    let key = (ns.clone(), f.name.clone());
                    fn_map.entry(key).or_insert_with(|| {
                        let name = short_fn_ident(fn_idx);
                        fn_idx += 1;
                        name
                    });
                }
            }
        }
    }

    for prog in programs.iter_mut() {
        let ns = prog.items.iter().find_map(|it| match it {
            Item::Namespace(n, _) => Some(n.clone()),
            _ => None,
        });
        let mut top_vars = VarMangler::new();

        for item in &mut prog.items {
            match item {
                Item::Function(f) => {
                    if let Some(new_name) = fn_map.get(&(ns.clone(), f.name.clone())) {
                        f.name = new_name.clone();
                    }
                    let mut m = VarMangler::new();
                    for p in &mut f.params {
                        p.name = m.get_or_insert(&p.name);
                        if let Some(def) = &mut p.default {
                            mangle_expr(def, &mut m, &ns, &fn_map);
                        }
                    }
                    mangle_stmts(&mut f.body, &mut m, &ns, &fn_map);
                }
                Item::Class(cls) => {
                    if let Some(ctor) = &mut cls.constructor {
                        let mut m = VarMangler::new();
                        for p in &mut ctor.params {
                            p.name = m.get_or_insert(&p.name);
                            if let Some(def) = &mut p.default {
                                mangle_expr(def, &mut m, &ns, &fn_map);
                            }
                        }
                        mangle_stmts(&mut ctor.body, &mut m, &ns, &fn_map);
                    }
                    for method in &mut cls.methods {
                        let mut m = VarMangler::new();
                        for p in &mut method.params {
                            p.name = m.get_or_insert(&p.name);
                            if let Some(def) = &mut p.default {
                                mangle_expr(def, &mut m, &ns, &fn_map);
                            }
                        }
                        mangle_stmts(&mut method.body, &mut m, &ns, &fn_map);
                    }
                }
                Item::Stmt(s) => {
                    mangle_stmt(s, &mut top_vars, &ns, &fn_map);
                }
                _ => {}
            }
        }
    }
}

struct VarMangler {
    map: HashMap<String, String>,
    used: HashSet<String>,
    counter: usize,
}

impl VarMangler {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            used: HashSet::new(),
            counter: 0,
        }
    }

    fn get_or_insert(&mut self, orig: &str) -> String {
        if is_reserved_var(orig) {
            return orig.to_string();
        }
        if let Some(existing) = self.map.get(orig) {
            return existing.clone();
        }
        loop {
            let candidate = short_ident(self.counter);
            self.counter += 1;
            if !self.used.contains(&candidate) {
                self.used.insert(candidate.clone());
                self.map.insert(orig.to_string(), candidate.clone());
                return candidate;
            }
        }
    }

    fn lookup(&self, orig: &str) -> Option<String> {
        self.map.get(orig).cloned()
    }
}

fn mangle_stmts(
    stmts: &mut [Stmt],
    m: &mut VarMangler,
    ns: &Option<String>,
    fn_map: &HashMap<(Option<String>, String), String>,
) {
    for s in stmts {
        mangle_stmt(s, m, ns, fn_map);
    }
}

fn mangle_stmt(
    stmt: &mut Stmt,
    m: &mut VarMangler,
    ns: &Option<String>,
    fn_map: &HashMap<(Option<String>, String), String>,
) {
    match stmt {
        Stmt::VarDecl { name, init, .. } => {
            mangle_expr(init, m, ns, fn_map);
            *name = m.get_or_insert(name);
        }
        Stmt::ConstDecl { init, .. } => {
            mangle_expr(init, m, ns, fn_map);
        }
        Stmt::Assign { target, value, .. } => {
            if let Expr::Var(vname, _) = target {
                *vname = m.get_or_insert(vname);
            } else {
                mangle_expr(target, m, ns, fn_map);
            }
            mangle_expr(value, m, ns, fn_map);
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                mangle_expr(v, m, ns, fn_map);
            }
        }
        Stmt::Echo { value, .. } => {
            mangle_expr(value, m, ns, fn_map);
        }
        Stmt::Global { vars, .. } => {
            for v in vars {
                m.map.insert(v.clone(), v.clone());
            }
        }
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            mangle_expr(cond, m, ns, fn_map);
            mangle_stmts(then_branch, m, ns, fn_map);
            if let Some(eb) = else_branch {
                mangle_stmts(eb, m, ns, fn_map);
            }
        }
        Stmt::While { cond, body, .. } => {
            mangle_expr(cond, m, ns, fn_map);
            mangle_stmts(body, m, ns, fn_map);
        }
        Stmt::ForEach {
            iterable,
            key_var,
            val_var,
            body,
            ..
        } => {
            mangle_expr(iterable, m, ns, fn_map);
            if let Some(kv) = key_var {
                *kv = m.get_or_insert(kv);
            }
            *val_var = m.get_or_insert(val_var);
            mangle_stmts(body, m, ns, fn_map);
        }
        Stmt::ExprStmt { expr, .. } => {
            mangle_expr(expr, m, ns, fn_map);
        }
    }
}

fn mangle_expr(
    expr: &mut Expr,
    m: &mut VarMangler,
    ns: &Option<String>,
    fn_map: &HashMap<(Option<String>, String), String>,
) {
    match expr {
        Expr::Var(name, _) => {
            if !is_reserved_var(name) {
                if let Some(mapped) = m.lookup(name) {
                    *name = mapped;
                }
            }
        }
        Expr::Ident(name, _) => {
            if let Some(mapped) = fn_map.get(&(ns.clone(), name.clone())) {
                *name = mapped.clone();
            }
        }
        Expr::ArrayLit(items, _) => {
            for it in items {
                mangle_expr(it, m, ns, fn_map);
            }
        }
        Expr::AssocArrayLit(entries, _) => {
            for (_, v) in entries {
                mangle_expr(v, m, ns, fn_map);
            }
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::InstanceOf { expr, .. } => {
            mangle_expr(expr, m, ns, fn_map);
        }
        Expr::Binary { left, right, .. } => {
            mangle_expr(left, m, ns, fn_map);
            mangle_expr(right, m, ns, fn_map);
        }
        Expr::Call { callee, args, .. } => {
            mangle_expr(callee, m, ns, fn_map);
            for a in args {
                mangle_expr(a, m, ns, fn_map);
            }
        }
        Expr::New { args, .. } => {
            for a in args {
                mangle_expr(a, m, ns, fn_map);
            }
        }
        Expr::Member { object, .. } => {
            mangle_expr(object, m, ns, fn_map);
        }
        Expr::Index { object, index, .. } => {
            mangle_expr(object, m, ns, fn_map);
            mangle_expr(index, m, ns, fn_map);
        }
        Expr::ArrowFn { params, body, .. } => {
            let saved_map = m.map.clone();
            let saved_used = m.used.clone();
            let saved_counter = m.counter;
            for p in params {
                p.name = m.get_or_insert(&p.name);
            }
            mangle_expr(body, m, ns, fn_map);
            m.map = saved_map;
            m.used = saved_used;
            m.counter = saved_counter;
        }
        Expr::Int(..)
        | Expr::Float(..)
        | Expr::Bool(..)
        | Expr::Null(..)
        | Expr::StaticAccess { .. } => {}
        Expr::String(s, _) => {
            if looks_like_html_or_css_or_js(s) {
                *s = minify_html_css_js(s);
            }
        }
    }
}

fn looks_like_html_or_css_or_js(s: &str) -> bool {
    let trimmed = s.trim_start();
    trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<div")
        || trimmed.starts_with("<article")
        || trimmed.starts_with("<section")
        || trimmed.starts_with("<script")
        || trimmed.starts_with("<style")
        || s.contains("<script>")
        || s.contains("</div")
        || s.contains("</article")
}

fn minify_html_css_js(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;

    while !rest.is_empty() {
        let next_script = find_tag_start(rest, "<script");
        let next_style = find_tag_start(rest, "<style");

        let next_tag = match (next_script, next_style) {
            (Some(a), Some(b)) if a <= b => Some((a, "</script>", true)),
            (Some(_), Some(b)) => Some((b, "</style>", false)),
            (Some(a), None) => Some((a, "</script>", true)),
            (None, Some(b)) => Some((b, "</style>", false)),
            (None, None) => None,
        };

        if let Some((start, close_tag, is_js)) = next_tag {
            let (before, after_start) = rest.split_at(start);
            out.push_str(&minify_html_fragment(before));
            if let Some(tag_end) = after_start.find('>') {
                let (open_tag, body_and_after) = after_start.split_at(tag_end + 1);
                out.push_str(open_tag);
                if let Some(close_idx) = find_tag_start(body_and_after, close_tag) {
                    let (inner, after_close) = body_and_after.split_at(close_idx);
                    out.push_str(&if is_js {
                        minify_js(inner)
                    } else {
                        minify_css(inner)
                    });
                    out.push_str(close_tag);
                    rest = &after_close[close_tag.len()..];
                    continue;
                } else {
                    out.push_str(&if is_js {
                        minify_js(body_and_after)
                    } else {
                        minify_css(body_and_after)
                    });
                    break;
                }
            }
        }

        out.push_str(&minify_html_fragment(rest));
        break;
    }

    out
}

fn find_tag_start(haystack: &str, needle: &str) -> Option<usize> {
    haystack.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

fn minify_html_fragment(src: &str) -> String {
    let mut no_comments = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("<!--") {
        no_comments.push_str(&rest[..start]);
        if let Some(end) = rest[start + 4..].find("-->") {
            rest = &rest[start + 4 + end + 3..];
        } else {
            rest = "";
            break;
        }
    }
    no_comments.push_str(rest);

    let mut collapsed = String::with_capacity(no_comments.len());
    let mut prev_ws = false;
    for ch in no_comments.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                collapsed.push(' ');
                prev_ws = true;
            }
        } else {
            collapsed.push(ch);
            prev_ws = false;
        }
    }

    collapsed.replace("> <", "><").trim().to_string()
}

fn minify_css(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    let mut pending_space = false;

    while i < len {
        let ch = chars[i];
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if ch.is_whitespace() {
            pending_space = true;
            i += 1;
            continue;
        }
        if pending_space {
            if let Some(prev) = out.chars().last() {
                if !matches!(prev, '{' | '}' | ':' | ';' | ',')
                    && !matches!(ch, '{' | '}' | ':' | ';' | ',')
                {
                    out.push(' ');
                }
            }
            pending_space = false;
        }
        out.push(ch);
        i += 1;
    }
    out
}

fn minify_js(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    let mut pending_space = false;
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' || ch == '`' {
            if pending_space {
                if let Some(prev) = out.chars().last() {
                    if is_word(prev) {
                        out.push(' ');
                    }
                }
                pending_space = false;
            }
            let quote = ch;
            out.push(quote);
            i += 1;
            while i < len {
                let c = chars[i];
                out.push(c);
                i += 1;
                if c == '\\' && i < len {
                    out.push(chars[i]);
                    i += 1;
                } else if c == quote {
                    break;
                }
            }
            continue;
        }
        if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            pending_space = true;
            continue;
        }
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(len);
            pending_space = true;
            continue;
        }
        if ch.is_whitespace() {
            pending_space = true;
            i += 1;
            continue;
        }
        if pending_space {
            if let Some(prev) = out.chars().last() {
                if is_word(prev) && is_word(ch) {
                    out.push(' ');
                }
            }
            pending_space = false;
        }
        out.push(ch);
        i += 1;
    }
    out
}
