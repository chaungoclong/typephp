use std::collections::{HashMap, HashSet, VecDeque};

use crate::ast::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhpVersion {
    Php81,
    Php82,
    Php83,
    Php84,
}

impl PhpVersion {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "8.1" => Ok(PhpVersion::Php81),
            "8.2" => Ok(PhpVersion::Php82),
            "8.3" => Ok(PhpVersion::Php83),
            "8.4" => Ok(PhpVersion::Php84),
            other => Err(format!(
                "Unsupported PHP target '{}'. Supported: 8.1, 8.2, 8.3, 8.4",
                other
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PhpVersion::Php81 => "8.1",
            PhpVersion::Php82 => "8.2",
            PhpVersion::Php83 => "8.3",
            PhpVersion::Php84 => "8.4",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitMode {
    Preserve,
    Bundle,
}

pub struct Codegen {
    out: String,
    indent: usize,
    type_aliases: HashMap<String, TypeExpr>,
    target: PhpVersion,
}

impl Codegen {
    pub fn new() -> Self {
        Self::new_with_target(PhpVersion::Php84)
    }

    pub fn new_with_target(target: PhpVersion) -> Self {
        Self {
            out: String::new(),
            indent: 0,
            type_aliases: HashMap::new(),
            target,
        }
    }

    pub fn emit_bundle(&mut self, modules: &[&Program]) -> String {
        for program in modules {
            for item in &program.items {
                if let Item::TypeAlias(ta) = item {
                    self.type_aliases.insert(ta.name.clone(), ta.ty.clone());
                }
            }
        }

        let has_any_namespace = modules.iter().any(|m| {
            m.items
                .iter()
                .any(|it| matches!(it, Item::Namespace(_, _)))
        });

        self.line("<?php");
        self.out.push('\n');
        self.line("declare(strict_types=1);");
        self.line(&format!("// Target: PHP {}", self.target.as_str()));
        self.out.push('\n');

        let reachable = compute_reachable_symbols(modules);
        let last_idx = modules.len().saturating_sub(1);

        for (mod_idx, program) in modules.iter().enumerate() {
            let is_reachable_item = |item: &Item| -> bool {
                let Some(ref r) = reachable else {
                    return true;
                };
                match item {
                    Item::Interface(i) => r.contains(&i.name),
                    Item::Enum(e) => r.contains(&e.name),
                    Item::Class(c) => r.contains(&c.name),
                    Item::Function(f) => r.contains(&f.name),
                    Item::Stmt(_) => true,
                    Item::Import { path, .. } | Item::Require(path, _) => !path.ends_with(".tphp"),
                    _ => false,
                }
            };

            let is_entry = mod_idx == last_idx;
            let auto_main = if is_entry {
                entry_auto_main_call(program)
            } else {
                None
            };

            let has_emitted_body = program.items.iter().any(is_reachable_item) || auto_main.is_some();
            if has_any_namespace && !has_emitted_body {
                continue;
            }

            let ns = program.items.iter().find_map(|it| match it {
                Item::Namespace(name, _) => Some(name.as_str()),
                _ => None,
            });

            if has_any_namespace {
                match ns {
                    Some(name) => self.line(&format!("namespace {}", name)),
                    None => self.line("namespace"),
                }
                self.line("{");
                self.indent += 1;
            }

            let mut used_symbols = HashSet::new();
            for item in &program.items {
                if is_reachable_item(item) {
                    collect_item_symbols(item, &mut used_symbols);
                }
            }
            let mut seen_uses = HashSet::new();
            let mut prev_was_use = false;
            for item in &program.items {
                match item {
                    Item::Namespace(_, _) | Item::TypeAlias(_) => {}
                    Item::Import { path, .. } | Item::Require(path, _) => {
                        if !path.ends_with(".tphp") {
                            let rel = path.strip_prefix("./").unwrap_or(path);
                            self.line(&format!(
                                "require_once __DIR__ . {};",
                                quote_php_string(&format!("/{}", rel))
                            ));
                        }
                    }
                    Item::Use(u, _) => {
                        let short = u.rsplit('\\').next().unwrap_or(u);
                        if !used_symbols.contains(short) || !seen_uses.insert(u.clone()) {
                            continue;
                        }
                        self.line(&format!("use {};", u));
                        prev_was_use = true;
                    }
                    Item::Interface(iface) => {
                        if !is_reachable_item(item) {
                            continue;
                        }
                        if prev_was_use {
                            self.out.push('\n');
                            prev_was_use = false;
                        }
                        self.emit_interface(iface);
                        self.out.push('\n');
                    }
                    Item::Enum(en) => {
                        if !is_reachable_item(item) {
                            continue;
                        }
                        if prev_was_use {
                            self.out.push('\n');
                            prev_was_use = false;
                        }
                        self.emit_enum(en);
                        self.out.push('\n');
                    }
                    Item::Class(cls) => {
                        if !is_reachable_item(item) {
                            continue;
                        }
                        if prev_was_use {
                            self.out.push('\n');
                            prev_was_use = false;
                        }
                        self.emit_class(cls);
                        self.out.push('\n');
                    }
                    Item::Function(f) => {
                        if !is_reachable_item(item) {
                            continue;
                        }
                        if prev_was_use {
                            self.out.push('\n');
                            prev_was_use = false;
                        }
                        self.emit_fn(f);
                        self.out.push('\n');
                    }
                    Item::Stmt(s) => {
                        if prev_was_use {
                            self.out.push('\n');
                            prev_was_use = false;
                        }
                        self.emit_stmt(s);
                    }
                }
            }

            if let Some(call_stmt) = auto_main {
                for line in call_stmt.lines() {
                    self.line(line);
                }
            }

            if has_any_namespace {
                self.indent -= 1;
                self.line("}");
                self.out.push('\n');
            }
        }

        while self.out.ends_with("\n\n") {
            self.out.pop();
        }
        self.out.clone()
    }

    pub fn emit_program(&mut self, program: &Program) -> String {
        self.emit_program_with_aliases(program, &HashMap::new())
    }

    pub fn emit_program_with_aliases(
        &mut self,
        program: &Program,
        external_aliases: &HashMap<String, TypeExpr>,
    ) -> String {
        self.emit_program_with_aliases_and_entry(program, external_aliases, true)
    }

    pub fn emit_program_with_aliases_and_entry(
        &mut self,
        program: &Program,
        external_aliases: &HashMap<String, TypeExpr>,
        is_entry: bool,
    ) -> String {
        for (k, v) in external_aliases {
            self.type_aliases.insert(k.clone(), v.clone());
        }
        for item in &program.items {
            if let Item::TypeAlias(ta) = item {
                self.type_aliases.insert(ta.name.clone(), ta.ty.clone());
            }
        }

        // PSR-12 header: <?php + blank line + declare(strict_types=1);
        self.line("<?php");
        self.out.push('\n');
        self.line("declare(strict_types=1);");
        self.line(&format!("// Target: PHP {}", self.target.as_str()));
        self.out.push('\n');

        let used_symbols = collect_used_symbols(program);
        let mut seen_uses = HashSet::new();
        let mut seen_requires = HashSet::new();

        let mut prev_was_use = false;
        let mut prev_was_req = false;

        for item in &program.items {
            match item {
                Item::Namespace(ns, _) => {
                    self.line(&format!("namespace {};", ns));
                    self.out.push('\n');
                    prev_was_use = false;
                    prev_was_req = false;
                }
                Item::Use(u, _) => {
                    let short = u.rsplit('\\').next().unwrap_or(u);
                    if !used_symbols.contains(short) || !seen_uses.insert(u.clone()) {
                        continue;
                    }
                    if prev_was_req {
                        self.out.push('\n');
                        prev_was_req = false;
                    }
                    self.line(&format!("use {};", u));
                    prev_was_use = true;
                }
                Item::Require(path, _) => {
                    let php_path = path.replace(".tphp", ".php");
                    let rel = php_path.strip_prefix("./").unwrap_or(&php_path).to_string();
                    if !seen_requires.insert(rel.clone()) {
                        continue;
                    }
                    if prev_was_use {
                        self.out.push('\n');
                        prev_was_use = false;
                    }
                    self.line(&format!(
                        "require_once __DIR__ . {};",
                        quote_php_string(&format!("/{}", rel))
                    ));
                    prev_was_req = true;
                }
                Item::Import { names, path, .. } => {
                    // Skip runtime require_once if all imported names are erased type aliases
                    if !names.is_empty()
                        && names.iter().all(|n| self.type_aliases.contains_key(n))
                    {
                        continue;
                    }
                    let php_path = path.replace(".tphp", ".php");
                    let rel = php_path.strip_prefix("./").unwrap_or(&php_path).to_string();
                    if !seen_requires.insert(rel.clone()) {
                        continue;
                    }
                    if prev_was_use {
                        self.out.push('\n');
                        prev_was_use = false;
                    }
                    self.line(&format!(
                        "require_once __DIR__ . {};",
                        quote_php_string(&format!("/{}", rel))
                    ));
                    prev_was_req = true;
                }
                Item::TypeAlias(_) => {}
                Item::Interface(iface) => {
                    if prev_was_use || prev_was_req {
                        self.out.push('\n');
                        prev_was_use = false;
                        prev_was_req = false;
                    }
                    self.emit_interface(iface);
                    self.out.push('\n');
                }
                Item::Enum(en) => {
                    if prev_was_use || prev_was_req {
                        self.out.push('\n');
                        prev_was_use = false;
                        prev_was_req = false;
                    }
                    self.emit_enum(en);
                    self.out.push('\n');
                }
                Item::Class(cls) => {
                    if prev_was_use || prev_was_req {
                        self.out.push('\n');
                        prev_was_use = false;
                        prev_was_req = false;
                    }
                    self.emit_class(cls);
                    self.out.push('\n');
                }
                Item::Function(f) => {
                    if prev_was_use || prev_was_req {
                        self.out.push('\n');
                        prev_was_use = false;
                        prev_was_req = false;
                    }
                    self.emit_fn(f);
                    self.out.push('\n');
                }
                Item::Stmt(s) => {
                    if prev_was_use || prev_was_req {
                        self.out.push('\n');
                        prev_was_use = false;
                        prev_was_req = false;
                    }
                    self.emit_stmt(s);
                }
            }
        }

        if is_entry {
            if let Some(call_stmt) = entry_auto_main_call(program) {
                for line in call_stmt.lines() {
                    self.line(line);
                }
            }
        }

        // PSR-12: single trailing newline at EOF
        while self.out.ends_with("\n\n") {
            self.out.pop();
        }
        self.out.clone()
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn emit_interface(&mut self, iface: &InterfaceDecl) {
        let ext = if iface.extends.is_empty() {
            String::new()
        } else {
            format!(" extends {}", iface.extends.join(", "))
        };
        self.line(&format!("interface {}{}", iface.name, ext));
        self.line("{");
        self.indent += 1;
        let empty = HashSet::new();
        for m in &iface.methods {
            self.emit_fn_phpdoc(&[], &m.params, Some(&m.return_type), &empty);
            let vis = vis_str(m.visibility);
            let params = self.format_params(&m.params, &empty);
            let ret = self.format_php_type(&m.return_type, &empty);
            self.line(&format!("{} function {}({}): {};", vis, m.name, params, ret));
        }
        self.indent -= 1;
        self.line("}");
    }

    fn emit_enum(&mut self, en: &EnumDecl) {
        let backing = match &en.backing_type {
            Some(b) => format!(": {}", b),
            None => String::new(),
        };
        self.line(&format!("enum {}{}", en.name, backing));
        self.line("{");
        self.indent += 1;
        for c in &en.cases {
            if let Some(val) = &c.value {
                self.line(&format!("case {} = {};", c.name, self.format_expr(val)));
            } else {
                self.line(&format!("case {};", c.name));
            }
        }
        self.indent -= 1;
        self.line("}");
    }

    fn emit_class(&mut self, cls: &ClassDecl) {
        let tparams: HashSet<String> = cls.type_params.iter().cloned().collect();
        if !cls.type_params.is_empty() {
            self.line("/**");
            for tp in &cls.type_params {
                self.line(&format!(" * @template {}", tp));
            }
            self.line(" */");
        }

        let mut header = format!("class {}", cls.name);
        if let Some(ext) = &cls.extends {
            header.push_str(&format!(" extends {}", ext));
        }
        if !cls.implements.is_empty() {
            header.push_str(&format!(" implements {}", cls.implements.join(", ")));
        }
        self.line(&header);
        self.line("{");
        self.indent += 1;

        for f in &cls.fields {
            let php_ty = self.format_php_type(&f.ty, &tparams);
            let stan_ty = self.format_phpstan_type(&f.ty, &tparams);
            if stan_ty != php_ty {
                self.line(&format!("/** @var {} */", stan_ty));
            }
            let vis = vis_str(f.visibility);
            if let Some(def) = &f.default {
                self.line(&format!(
                    "{} {} ${} = {};",
                    vis,
                    php_ty,
                    f.name,
                    self.format_expr(def)
                ));
            } else {
                self.line(&format!("{} {} ${};", vis, php_ty, f.name));
            }
        }

        if !cls.fields.is_empty() && (cls.constructor.is_some() || !cls.methods.is_empty()) {
            self.out.push('\n');
        }

        if let Some(ctor) = &cls.constructor {
            self.emit_fn_phpdoc(&[], &ctor.params, None, &tparams);
            let vis = vis_str(ctor.visibility);
            let params = self.format_params(&ctor.params, &tparams);
            self.line(&format!("{} function __construct({})", vis, params));
            self.line("{");
            self.indent += 1;
            for s in &ctor.body {
                self.emit_stmt(s);
            }
            self.indent -= 1;
            self.line("}");
            if !cls.methods.is_empty() {
                self.out.push('\n');
            }
        }

        for (idx, m) in cls.methods.iter().enumerate() {
            let mut method_tparams = tparams.clone();
            method_tparams.extend(m.type_params.iter().cloned());
            self.emit_fn_phpdoc(&m.type_params, &m.params, Some(&m.return_type), &method_tparams);
            let vis = vis_str(m.visibility);
            let params = self.format_params(&m.params, &method_tparams);
            let ret = self.format_php_type(&m.return_type, &method_tparams);
            self.line(&format!("{} function {}({}): {}", vis, m.name, params, ret));
            self.line("{");
            self.indent += 1;
            for s in &m.body {
                self.emit_stmt(s);
            }
            self.indent -= 1;
            self.line("}");
            if idx + 1 < cls.methods.len() {
                self.out.push('\n');
            }
        }

        self.indent -= 1;
        self.line("}");
    }

    fn emit_fn(&mut self, f: &FnDecl) {
        let tparams: HashSet<String> = f.type_params.iter().cloned().collect();
        self.emit_fn_phpdoc(&f.type_params, &f.params, Some(&f.return_type), &tparams);
        let params = self.format_params(&f.params, &tparams);
        let ret = self.format_php_type(&f.return_type, &tparams);
        self.line(&format!("function {}({}): {}", f.name, params, ret));
        self.line("{");
        self.indent += 1;
        for s in &f.body {
            self.emit_stmt(s);
        }
        self.indent -= 1;
        self.line("}");
    }

    fn emit_fn_phpdoc(
        &mut self,
        type_params: &[String],
        params: &[Param],
        return_type: Option<&TypeExpr>,
        tparams: &HashSet<String>,
    ) {
        let mut lines = Vec::new();
        for tp in type_params {
            lines.push(format!(" * @template {}", tp));
        }
        for p in params {
            let php_ty = self.format_php_type(&p.ty, tparams);
            let stan_ty = self.format_phpstan_type(&p.ty, tparams);
            if stan_ty != php_ty {
                lines.push(format!(" * @param {} ${}", stan_ty, p.name));
            }
        }
        if let Some(rt) = return_type {
            let php_rt = self.format_php_type(rt, tparams);
            let stan_rt = self.format_phpstan_type(rt, tparams);
            if stan_rt != php_rt {
                lines.push(format!(" * @return {}", stan_rt));
            }
        }
        if !lines.is_empty() {
            self.line("/**");
            for l in lines {
                self.line(&l);
            }
            self.line(" */");
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl { name, ty, init, .. } => {
                if let Some(ty_expr) = ty {
                    let empty = HashSet::new();
                    let php_ty = self.format_php_type(ty_expr, &empty);
                    let stan_ty = self.format_phpstan_type(ty_expr, &empty);
                    if stan_ty != php_ty {
                        self.line(&format!("/** @var {} ${} */", stan_ty, name));
                    }
                }
                self.line(&format!("${} = {};", name, self.format_expr(init)));
            }
            Stmt::ConstDecl { name, init, .. } => {
                self.line(&format!("const {} = {};", name, self.format_expr(init)));
            }
            Stmt::Assign { target, value, .. } => {
                self.line(&format!(
                    "{} = {};",
                    self.format_expr(target),
                    self.format_expr(value)
                ));
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.line(&format!("return {};", self.format_expr(v)));
                } else {
                    self.line("return;");
                }
            }
            Stmt::Echo { value, .. } => {
                self.line(&format!("echo {};", self.format_expr(value)));
            }
            Stmt::Global { vars, .. } => {
                let list: Vec<String> = vars.iter().map(|v| format!("${}", v)).collect();
                self.line(&format!("global {};", list.join(", ")));
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.line(&format!("if ({}) {{", self.format_expr(cond)));
                self.indent += 1;
                for s in then_branch {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                if let Some(else_stmts) = else_branch {
                    if else_stmts.len() == 1 {
                        if let Stmt::If { .. } = &else_stmts[0] {
                            self.emit_elseif(&else_stmts[0]);
                            return;
                        }
                    }
                    self.line("} else {");
                    self.indent += 1;
                    for s in else_stmts {
                        self.emit_stmt(s);
                    }
                    self.indent -= 1;
                }
                self.line("}");
            }
            Stmt::While { cond, body, .. } => {
                self.line(&format!("while ({}) {{", self.format_expr(cond)));
                self.indent += 1;
                for s in body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.line("}");
            }
            Stmt::ForEach {
                iterable,
                key_var,
                val_var,
                body,
                ..
            } => {
                let binding = match key_var {
                    Some(k) => format!("${} => ${}", k, val_var),
                    None => format!("${}", val_var),
                };
                self.line(&format!(
                    "foreach ({} as {}) {{",
                    self.format_expr(iterable),
                    binding
                ));
                self.indent += 1;
                for s in body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.line("}");
            }
            Stmt::ExprStmt { expr, .. } => {
                self.line(&format!("{};", self.format_expr(expr)));
            }
        }
    }

    fn emit_elseif(&mut self, stmt: &Stmt) {
        if let Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } = stmt
        {
            self.line(&format!("}} elseif ({}) {{", self.format_expr(cond)));
            self.indent += 1;
            for s in then_branch {
                self.emit_stmt(s);
            }
            self.indent -= 1;
            if let Some(else_stmts) = else_branch {
                if else_stmts.len() == 1 {
                    if let Stmt::If { .. } = &else_stmts[0] {
                        self.emit_elseif(&else_stmts[0]);
                        return;
                    }
                }
                self.line("} else {");
                self.indent += 1;
                for s in else_stmts {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
            }
            self.line("}");
        }
    }

    fn format_params(&self, params: &[Param], tparams: &HashSet<String>) -> String {
        params
            .iter()
            .map(|p| {
                let ty = self.format_php_type(&p.ty, tparams);
                if let Some(def) = &p.default {
                    format!("{} ${} = {}", ty, p.name, self.format_expr(def))
                } else {
                    format!("{} ${}", ty, p.name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn format_php_type(&self, ty: &TypeExpr, tparams: &HashSet<String>) -> String {
        match ty {
            TypeExpr::Named(name, _, _) => {
                if tparams.contains(name) {
                    return "mixed".to_string();
                }
                if let Some(aliased) = self.type_aliases.get(name) {
                    return self.format_php_type(aliased, tparams);
                }
                match name.as_str() {
                    "any" => "mixed".to_string(),
                    "dict" => "array".to_string(),
                    other => other.to_string(),
                }
            }
            TypeExpr::Array(_, _) | TypeExpr::Shape(_, _) => "array".to_string(),
            TypeExpr::Optional(inner, _) => {
                let inner_str = self.format_php_type(inner, tparams);
                if inner_str == "mixed" {
                    "mixed".to_string()
                } else {
                    format!("?{}", inner_str)
                }
            }
            TypeExpr::Union(items, _) => {
                let mut parts: Vec<String> = items
                    .iter()
                    .map(|t| self.format_php_type(t, tparams))
                    .collect();
                parts.dedup();
                if parts.contains(&"mixed".to_string()) {
                    "mixed".to_string()
                } else {
                    parts.join("|")
                }
            }
            TypeExpr::Function { .. } => "\\Closure".to_string(),
        }
    }

    fn format_phpstan_type(&self, ty: &TypeExpr, tparams: &HashSet<String>) -> String {
        match ty {
            TypeExpr::Named(name, args, _) => {
                if tparams.contains(name) {
                    return name.clone();
                }
                if let Some(aliased) = self.type_aliases.get(name) {
                    return self.format_phpstan_type(aliased, tparams);
                }
                match name.as_str() {
                    "any" => "mixed".to_string(),
                    "array" => {
                        if args.len() == 1 {
                            format!("list<{}>", self.format_phpstan_type(&args[0], tparams))
                        } else {
                            "list<mixed>".to_string()
                        }
                    }
                    "dict" => {
                        if args.len() == 2 {
                            format!(
                                "array<{}, {}>",
                                self.format_phpstan_type(&args[0], tparams),
                                self.format_phpstan_type(&args[1], tparams)
                            )
                        } else {
                            "array<string, mixed>".to_string()
                        }
                    }
                    other => other.to_string(),
                }
            }
            TypeExpr::Array(elem, _) => {
                format!("list<{}>", self.format_phpstan_type(elem, tparams))
            }
            TypeExpr::Optional(inner, _) => {
                format!("{}|null", self.format_phpstan_type(inner, tparams))
            }
            TypeExpr::Union(items, _) => items
                .iter()
                .map(|t| self.format_phpstan_type(t, tparams))
                .collect::<Vec<_>>()
                .join("|"),
            TypeExpr::Shape(fields, _) => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let opt = if f.optional { "?" } else { "" };
                        format!(
                            "{}{}: {}",
                            f.name,
                            opt,
                            self.format_phpstan_type(&f.ty, tparams)
                        )
                    })
                    .collect();
                format!("array{{{}}}", parts.join(", "))
            }
            TypeExpr::Function { params, ret, .. } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| self.format_phpstan_type(p, tparams))
                    .collect();
                format!(
                    "\\Closure({}): {}",
                    ps.join(", "),
                    self.format_phpstan_type(ret, tparams)
                )
            }
        }
    }

    fn format_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Int(v, _) => v.to_string(),
            Expr::Float(v, _) => {
                let s = v.to_string();
                if s.contains('.') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
            Expr::String(s, _) => quote_php_string(s),
            Expr::Bool(b, _) => if *b { "true" } else { "false" }.to_string(),
            Expr::Null(_) => "null".to_string(),
            Expr::Var(name, _) => format!("${}", name),
            Expr::Ident(name, _) => name.clone(),
            Expr::ArrayLit(items, _) => {
                let inner: Vec<String> = items.iter().map(|e| self.format_expr(e)).collect();
                format!("[{}]", inner.join(", "))
            }
            Expr::AssocArrayLit(entries, _) => {
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{} => {}", quote_php_string(k), self.format_expr(v)))
                    .collect();
                format!("[{}]", inner.join(", "))
            }
            Expr::Unary { op, expr, .. } => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                };
                format!("{}{}", op_str, self.format_expr(expr))
            }
            Expr::Cast { target_ty, expr, .. } => {
                format!("({}) {}", target_ty, self.format_expr(expr))
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::Concat => ".",
                    BinaryOp::EqEqEq => "===",
                    BinaryOp::NotEqEq => "!==",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::AndAnd => "&&",
                    BinaryOp::OrOr => "||",
                    BinaryOp::NullCoal => "??",
                };
                format!(
                    "{} {} {}",
                    self.format_expr(left),
                    op_str,
                    self.format_expr(right)
                )
            }
            Expr::InstanceOf {
                expr, class_name, ..
            } => format!("{} instanceof {}", self.format_expr(expr), class_name),
            Expr::Call { callee, args, .. } => {
                let arg_str = args
                    .iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", self.format_expr(callee), arg_str)
            }
            Expr::New {
                class_name, args, ..
            } => {
                let arg_str = args
                    .iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("new {}({})", class_name, arg_str)
            }
            Expr::Member {
                object, property, ..
            } => format!("{}->{}", self.format_expr(object), property),
            Expr::StaticAccess {
                class_name, member, ..
            } => format!("{}::{}", class_name, member),
            Expr::Index { object, index, .. } => {
                format!("{}[{}]", self.format_expr(object), self.format_expr(index))
            }
            Expr::ArrowFn {
                params,
                return_type,
                body,
                ..
            } => {
                let empty = HashSet::new();
                let pstr = self.format_params(params, &empty);
                let rstr = match return_type {
                    Some(rt) => format!(": {}", self.format_php_type(rt, &empty)),
                    None => String::new(),
                };
                format!("fn ({}){} => {}", pstr, rstr, self.format_expr(body))
            }
        }
    }
}

fn vis_str(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "public",
        Visibility::Private => "private",
        Visibility::Protected => "protected",
    }
}

fn quote_php_string(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{}'", escaped)
}

fn is_string_array_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Array(inner, _) if matches!(inner.as_ref(), TypeExpr::Named(n, _, _) if n == "string"))
}

fn entry_auto_main_call(program: &Program) -> Option<String> {
    let main_fn = program.items.iter().find_map(|it| match it {
        Item::Function(f) if f.name == "main" => Some(f),
        _ => None,
    })?;
    let already_called = program.items.iter().any(|it| match it {
        Item::Stmt(s) => stmt_contains_fn_call(s, "main"),
        _ => false,
    });
    if already_called {
        return None;
    }

    let returns_int = matches!(&main_fn.return_type, TypeExpr::Named(n, _, _) if n == "int");
    let wrap = |call: &str| -> String {
        if returns_int {
            format!("exit({});", call)
        } else {
            format!("{};", call)
        }
    };

    if main_fn.params.iter().all(|p| p.default.is_some()) {
        return Some(wrap("main()"));
    }

    let argv_expr = "array_values(array_filter(is_array($_SERVER['argv'] ?? null) ? $_SERVER['argv'] : [], fn (mixed $a): bool => is_string($a)))";

    if main_fn.params.len() == 1 && is_string_array_type(&main_fn.params[0].ty) {
        return Some(wrap(&format!("main({})", argv_expr)));
    }

    if main_fn.params.len() == 2
        && matches!(&main_fn.params[0].ty, TypeExpr::Named(n, _, _) if n == "int")
        && is_string_array_type(&main_fn.params[1].ty)
    {
        return Some(format!(
            "$__tphp_argv = {};\n{}",
            argv_expr,
            wrap("main(count($__tphp_argv), $__tphp_argv)")
        ));
    }

    None
}

fn stmt_contains_fn_call(stmt: &Stmt, fn_name: &str) -> bool {
    match stmt {
        Stmt::VarDecl { init, .. }
        | Stmt::ConstDecl { init, .. }
        | Stmt::Assign { value: init, .. }
        | Stmt::Echo { value: init, .. }
        | Stmt::ExprStmt { expr: init, .. } => expr_contains_fn_call_cg(init, fn_name),
        Stmt::Return { value: Some(v), .. } => expr_contains_fn_call_cg(v, fn_name),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_contains_fn_call_cg(cond, fn_name)
                || then_branch.iter().any(|s| stmt_contains_fn_call(s, fn_name))
                || else_branch
                    .as_ref()
                    .is_some_and(|eb| eb.iter().any(|s| stmt_contains_fn_call(s, fn_name)))
        }
        Stmt::While { cond, body, .. } | Stmt::ForEach { iterable: cond, body, .. } => {
            expr_contains_fn_call_cg(cond, fn_name)
                || body.iter().any(|s| stmt_contains_fn_call(s, fn_name))
        }
        _ => false,
    }
}

fn expr_contains_fn_call_cg(expr: &Expr, fn_name: &str) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Expr::Ident(n, _) = callee.as_ref() {
                if n.trim_start_matches('\\') == fn_name {
                    return true;
                }
            }
            expr_contains_fn_call_cg(callee, fn_name)
                || args.iter().any(|a| expr_contains_fn_call_cg(a, fn_name))
        }
        Expr::Binary { left, right, .. } => {
            expr_contains_fn_call_cg(left, fn_name) || expr_contains_fn_call_cg(right, fn_name)
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::InstanceOf { expr, .. } => {
            expr_contains_fn_call_cg(expr, fn_name)
        }
        Expr::ArrayLit(items, _) | Expr::New { args: items, .. } => {
            items.iter().any(|a| expr_contains_fn_call_cg(a, fn_name))
        }
        Expr::AssocArrayLit(entries, _) => entries
            .iter()
            .any(|(_, v)| expr_contains_fn_call_cg(v, fn_name)),
        Expr::Member { object, .. } => expr_contains_fn_call_cg(object, fn_name),
        Expr::Index { object, index, .. } => {
            expr_contains_fn_call_cg(object, fn_name) || expr_contains_fn_call_cg(index, fn_name)
        }
        Expr::ArrowFn { body, .. } => expr_contains_fn_call_cg(body, fn_name),
        _ => false,
    }
}

fn compute_reachable_symbols(modules: &[&Program]) -> Option<HashSet<String>> {
    let mut seeds = HashSet::new();
    let mut has_roots = false;

    for prog in modules {
        for item in &prog.items {
            if let Item::Stmt(s) = item {
                has_roots = true;
                collect_stmt_symbols(std::slice::from_ref(s), &mut seeds);
            }
        }
    }

    if let Some(entry) = modules.last() {
        if entry_auto_main_call(entry).is_some() {
            has_roots = true;
            seeds.insert("main".to_string());
        }
    }

    if !has_roots {
        return None;
    }

    let mut item_map: HashMap<String, Vec<&Item>> = HashMap::new();
    for prog in modules {
        for item in &prog.items {
            let name = match item {
                Item::Interface(i) => Some(i.name.clone()),
                Item::Enum(e) => Some(e.name.clone()),
                Item::Class(c) => Some(c.name.clone()),
                Item::Function(f) => Some(f.name.clone()),
                Item::TypeAlias(ta) => Some(ta.name.clone()),
                _ => None,
            };
            if let Some(n) = name {
                item_map.entry(n).or_default().push(item);
            }
        }
    }

    let mut reachable = HashSet::new();
    let mut queue: VecDeque<String> = seeds.into_iter().collect();

    while let Some(sym) = queue.pop_front() {
        let short = sym.rsplit('\\').next().unwrap_or(&sym).to_string();
        if !reachable.insert(short.clone()) {
            continue;
        }
        if let Some(items) = item_map.get(&short) {
            let mut deps = HashSet::new();
            for it in items {
                collect_item_symbols(it, &mut deps);
            }
            for d in deps {
                let d_short = d.rsplit('\\').next().unwrap_or(&d).to_string();
                if !reachable.contains(&d_short) {
                    queue.push_back(d_short);
                }
            }
        }
    }

    Some(reachable)
}

fn collect_used_symbols(program: &Program) -> HashSet<String> {
    let mut used = HashSet::new();
    for item in &program.items {
        collect_item_symbols(item, &mut used);
    }
    used
}

fn collect_item_symbols(item: &Item, used: &mut HashSet<String>) {
    match item {
        Item::TypeAlias(ta) => collect_type_symbols(&ta.ty, used),
        Item::Interface(iface) => {
            for ext in &iface.extends {
                used.insert(ext.clone());
            }
            for m in &iface.methods {
                for p in &m.params {
                    collect_type_symbols(&p.ty, used);
                }
                collect_type_symbols(&m.return_type, used);
            }
        }
        Item::Class(cls) => {
            if let Some(ext) = &cls.extends {
                used.insert(ext.clone());
            }
            for imp in &cls.implements {
                used.insert(imp.clone());
            }
            for f in &cls.fields {
                collect_type_symbols(&f.ty, used);
                if let Some(def) = &f.default {
                    collect_expr_symbols(def, used);
                }
            }
            if let Some(ctor) = &cls.constructor {
                for p in &ctor.params {
                    collect_type_symbols(&p.ty, used);
                    if let Some(def) = &p.default {
                        collect_expr_symbols(def, used);
                    }
                }
                collect_stmt_symbols(&ctor.body, used);
            }
            for m in &cls.methods {
                for p in &m.params {
                    collect_type_symbols(&p.ty, used);
                    if let Some(def) = &p.default {
                        collect_expr_symbols(def, used);
                    }
                }
                collect_type_symbols(&m.return_type, used);
                collect_stmt_symbols(&m.body, used);
            }
        }
        Item::Function(f) => {
            for p in &f.params {
                collect_type_symbols(&p.ty, used);
                if let Some(def) = &p.default {
                    collect_expr_symbols(def, used);
                }
            }
            collect_type_symbols(&f.return_type, used);
            collect_stmt_symbols(&f.body, used);
        }
        Item::Stmt(s) => collect_stmt_symbols(std::slice::from_ref(s), used),
        _ => {}
    }
}

fn collect_type_symbols(ty: &TypeExpr, used: &mut HashSet<String>) {
    match ty {
        TypeExpr::Named(name, args, _) => {
            used.insert(name.clone());
            for a in args {
                collect_type_symbols(a, used);
            }
        }
        TypeExpr::Array(inner, _) | TypeExpr::Optional(inner, _) => {
            collect_type_symbols(inner, used);
        }
        TypeExpr::Union(items, _) => {
            for t in items {
                collect_type_symbols(t, used);
            }
        }
        TypeExpr::Shape(fields, _) => {
            for f in fields {
                collect_type_symbols(&f.ty, used);
            }
        }
        TypeExpr::Function { params, ret, .. } => {
            for p in params {
                collect_type_symbols(p, used);
            }
            collect_type_symbols(ret, used);
        }
    }
}

fn collect_stmt_symbols(stmts: &[Stmt], used: &mut HashSet<String>) {
    for s in stmts {
        match s {
            Stmt::VarDecl { ty, init, .. } | Stmt::ConstDecl { ty, init, .. } => {
                if let Some(t) = ty {
                    collect_type_symbols(t, used);
                }
                collect_expr_symbols(init, used);
            }
            Stmt::Assign { target, value, .. } => {
                collect_expr_symbols(target, used);
                collect_expr_symbols(value, used);
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    collect_expr_symbols(v, used);
                }
            }
            Stmt::Echo { value, .. } => collect_expr_symbols(value, used),
            Stmt::Global { .. } => {}
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                collect_expr_symbols(cond, used);
                collect_stmt_symbols(then_branch, used);
                if let Some(eb) = else_branch {
                    collect_stmt_symbols(eb, used);
                }
            }
            Stmt::While { cond, body, .. } | Stmt::ForEach { iterable: cond, body, .. } => {
                collect_expr_symbols(cond, used);
                collect_stmt_symbols(body, used);
            }
            Stmt::ExprStmt { expr, .. } => collect_expr_symbols(expr, used),
        }
    }
}

fn collect_expr_symbols(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident(name, _) => {
            used.insert(name.clone());
        }
        Expr::New {
            class_name,
            type_args,
            args,
            ..
        } => {
            used.insert(class_name.clone());
            for ta in type_args {
                collect_type_symbols(ta, used);
            }
            for a in args {
                collect_expr_symbols(a, used);
            }
        }
        Expr::StaticAccess { class_name, .. } => {
            used.insert(class_name.clone());
        }
        Expr::InstanceOf {
            expr, class_name, ..
        } => {
            collect_expr_symbols(expr, used);
            used.insert(class_name.clone());
        }
        Expr::ArrayLit(items, _) => {
            for it in items {
                collect_expr_symbols(it, used);
            }
        }
        Expr::AssocArrayLit(entries, _) => {
            for (_, v) in entries {
                collect_expr_symbols(v, used);
            }
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } => collect_expr_symbols(expr, used),
        Expr::Binary { left, right, .. } => {
            collect_expr_symbols(left, used);
            collect_expr_symbols(right, used);
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_symbols(callee, used);
            for a in args {
                collect_expr_symbols(a, used);
            }
        }
        Expr::Member { object, .. } => collect_expr_symbols(object, used),
        Expr::Index { object, index, .. } => {
            collect_expr_symbols(object, used);
            collect_expr_symbols(index, used);
        }
        Expr::ArrowFn {
            params,
            return_type,
            body,
            ..
        } => {
            for p in params {
                collect_type_symbols(&p.ty, used);
            }
            if let Some(rt) = return_type {
                collect_type_symbols(rt, used);
            }
            collect_expr_symbols(body, used);
        }
        Expr::Int(..) | Expr::Float(..) | Expr::String(..) | Expr::Bool(..) | Expr::Null(..) | Expr::Var(..) => {}
    }
}

pub fn minify_php(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    let mut pending_space = false;

    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';

    while i < len {
        let ch = chars[i];

        // String literals: preserve verbatim
        if ch == '\'' || ch == '"' {
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

        // Line comments
        if (ch == '/' && i + 1 < len && chars[i + 1] == '/') || ch == '#' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            pending_space = true;
            continue;
        }

        // Block comments & PHPDoc
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
                let need_space = (is_word(prev) && is_word(ch))
                    || (prev.is_ascii_digit() && ch == '.')
                    || (prev == '.' && ch.is_ascii_digit())
                    || (prev == '?' && ch == '?');
                if need_space {
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
