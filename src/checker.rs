use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::{CompileError, CompileWarning, Span};
use crate::types::Type;

#[derive(Debug, Clone)]
struct FnSig {
    type_params: Vec<String>,
    params: Vec<(String, Type, bool)>, // (name, type, has_default)
    ret: Type,
}

#[derive(Debug, Clone)]
struct ClassInfo {
    type_params: Vec<String>,
    extends: Option<String>,
    implements: Vec<String>,
    fields: HashMap<String, (Visibility, Type)>,
    constructor: Option<Vec<(String, Type, bool)>>,
    methods: HashMap<String, (Visibility, FnSig)>,
}

#[derive(Debug, Clone)]
struct InterfaceInfo {
    extends: Vec<String>,
    methods: HashMap<String, FnSig>,
}

#[derive(Debug, Clone)]
struct EnumInfo {
    backing_type: Option<String>,
    cases: Vec<String>,
}

#[derive(Debug, Clone)]
struct TypeAliasInfo {
    type_params: Vec<String>,
    ty: TypeExpr,
}

pub struct TypeChecker {
    type_aliases: HashMap<String, TypeAliasInfo>,
    interfaces: HashMap<String, InterfaceInfo>,
    enums: HashMap<String, EnumInfo>,
    classes: HashMap<String, ClassInfo>,
    functions: HashMap<String, FnSig>,
    constants: HashMap<String, Type>,
    scopes: Vec<HashMap<String, Type>>,
    current_return: Option<Type>,
    current_class: Option<String>,
    resolving_aliases: RefCell<HashSet<String>>,
    pub warnings: Vec<CompileWarning>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            type_aliases: HashMap::new(),
            interfaces: HashMap::new(),
            enums: HashMap::new(),
            classes: HashMap::new(),
            functions: HashMap::new(),
            constants: HashMap::new(),
            scopes: vec![HashMap::new()],
            current_return: None,
            current_class: None,
            resolving_aliases: RefCell::new(HashSet::new()),
            warnings: Vec::new(),
        };
        checker.register_builtins();
        checker
    }

    fn register_builtins(&mut self) {
        let mut add_fn = |name: &str, params: Vec<Type>, ret: Type| {
            self.functions.insert(
                name.to_string(),
                FnSig {
                    type_params: vec![],
                    params: params
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| (format!("arg{}", i), t, false))
                        .collect(),
                    ret,
                },
            );
        };
        add_fn("strlen", vec![Type::String], Type::Int);
        add_fn("count", vec![Type::Mixed], Type::Int);
        add_fn("is_int", vec![Type::Mixed], Type::Bool);
        add_fn("is_string", vec![Type::Mixed], Type::Bool);
        add_fn("is_bool", vec![Type::Mixed], Type::Bool);
        add_fn("is_float", vec![Type::Mixed], Type::Bool);
        add_fn("is_array", vec![Type::Mixed], Type::Bool);
        add_fn("var_dump", vec![Type::Mixed], Type::Void);
        add_fn("print_r", vec![Type::Mixed], Type::Void);
        add_fn(
            "implode",
            vec![Type::String, Type::Array(Box::new(Type::String))],
            Type::String,
        );
        add_fn(
            "explode",
            vec![Type::String, Type::String],
            Type::Array(Box::new(Type::String)),
        );
        add_fn("strtoupper", vec![Type::String], Type::String);
        add_fn("strtolower", vec![Type::String], Type::String);
        add_fn("str_starts_with", vec![Type::String, Type::String], Type::Bool);
        add_fn("htmlspecialchars", vec![Type::String], Type::String);
        add_fn("intval", vec![Type::Mixed], Type::Int);
        add_fn("strval", vec![Type::Mixed], Type::String);
        add_fn("header", vec![Type::String], Type::Void);
        add_fn("file_get_contents", vec![Type::String], Type::String);
        add_fn("json_encode", vec![Type::Mixed], Type::String);
        add_fn(
            "array_key_exists",
            vec![Type::String, Type::Mixed],
            Type::Bool,
        );
        add_fn(
            "array_push",
            vec![Type::Array(Box::new(Type::Mixed)), Type::Mixed],
            Type::Int,
        );

        // Functions with optional parameters
        self.functions.insert(
            "trim".to_string(),
            FnSig {
                type_params: vec![],
                params: vec![
                    ("string".to_string(), Type::String, false),
                    ("characters".to_string(), Type::String, true),
                ],
                ret: Type::String,
            },
        );
        self.functions.insert(
            "substr".to_string(),
            FnSig {
                type_params: vec![],
                params: vec![
                    ("string".to_string(), Type::String, false),
                    ("offset".to_string(), Type::Int, false),
                    ("length".to_string(), Type::Int, true),
                ],
                ret: Type::String,
            },
        );
        self.functions.insert(
            "http_response_code".to_string(),
            FnSig {
                type_params: vec![],
                params: vec![("code".to_string(), Type::Int, true)],
                ret: Type::Int,
            },
        );
        self.functions.insert(
            "json_decode".to_string(),
            FnSig {
                type_params: vec![],
                params: vec![
                    ("json".to_string(), Type::String, false),
                    ("associative".to_string(), Type::Bool, true),
                ],
                ret: Type::Mixed,
            },
        );
        self.functions.insert(
            "parse_url".to_string(),
            FnSig {
                type_params: vec![],
                params: vec![
                    ("url".to_string(), Type::String, false),
                    ("component".to_string(), Type::Int, true),
                ],
                ret: Type::Mixed,
            },
        );

        // PHP built-in constants & superglobals
        self.constants.insert("PHP_URL_PATH".to_string(), Type::Int);
        self.constants.insert("__DIR__".to_string(), Type::String);
        self.declare_var(
            "_SERVER",
            Type::Dict(Box::new(Type::String), Box::new(Type::Mixed)),
        );
        self.declare_var(
            "_GET",
            Type::Dict(Box::new(Type::String), Box::new(Type::Mixed)),
        );
        self.declare_var(
            "_COOKIE",
            Type::Dict(Box::new(Type::String), Box::new(Type::Mixed)),
        );
        self.functions.insert(
            "setcookie".to_string(),
            FnSig {
                type_params: vec![],
                params: vec![
                    ("name".to_string(), Type::String, false),
                    ("value".to_string(), Type::String, true),
                    ("expires".to_string(), Type::Int, true),
                    ("path".to_string(), Type::String, true),
                ],
                ret: Type::Bool,
            },
        );

        // Built-in PDOStatement & PDO classes
        let mut stmt_methods = HashMap::new();
        stmt_methods.insert(
            "execute".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![(
                        "params".to_string(),
                        Type::Array(Box::new(Type::Mixed)),
                        true,
                    )],
                    ret: Type::Bool,
                },
            ),
        );
        stmt_methods.insert(
            "fetch".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![("mode".to_string(), Type::Int, true)],
                    ret: Type::Mixed,
                },
            ),
        );
        stmt_methods.insert(
            "fetchAll".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![("mode".to_string(), Type::Int, true)],
                    ret: Type::Dict(Box::new(Type::Int), Box::new(Type::Mixed)),
                },
            ),
        );
        stmt_methods.insert(
            "rowCount".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![],
                    ret: Type::Int,
                },
            ),
        );
        self.classes.insert(
            "PDOStatement".to_string(),
            ClassInfo {
                type_params: vec![],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                constructor: None,
                methods: stmt_methods,
            },
        );

        let mut pdo_methods = HashMap::new();
        pdo_methods.insert(
            "exec".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![("statement".to_string(), Type::String, false)],
                    ret: Type::Int,
                },
            ),
        );
        pdo_methods.insert(
            "prepare".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![("query".to_string(), Type::String, false)],
                    ret: Type::Class("PDOStatement".to_string(), vec![]),
                },
            ),
        );
        pdo_methods.insert(
            "query".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![("query".to_string(), Type::String, false)],
                    ret: Type::Class("PDOStatement".to_string(), vec![]),
                },
            ),
        );
        pdo_methods.insert(
            "lastInsertId".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![],
                    ret: Type::String,
                },
            ),
        );
        pdo_methods.insert(
            "setAttribute".to_string(),
            (
                Visibility::Public,
                FnSig {
                    type_params: vec![],
                    params: vec![
                        ("attribute".to_string(), Type::Int, false),
                        ("value".to_string(), Type::Mixed, false),
                    ],
                    ret: Type::Bool,
                },
            ),
        );
        self.classes.insert(
            "PDO".to_string(),
            ClassInfo {
                type_params: vec![],
                extends: None,
                implements: vec![],
                fields: HashMap::new(),
                constructor: Some(vec![("dsn".to_string(), Type::String, false)]),
                methods: pdo_methods,
            },
        );
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn lookup_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn set_narrowed_var(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), CompileError> {
        // Pass 0: pre-register interface and enum names for forward references
        for item in &program.items {
            match item {
                Item::Interface(iface) => {
                    self.interfaces.insert(
                        iface.name.clone(),
                        InterfaceInfo {
                            extends: iface.extends.clone(),
                            methods: HashMap::new(),
                        },
                    );
                }
                Item::Enum(en) => {
                    self.enums.insert(
                        en.name.clone(),
                        EnumInfo {
                            backing_type: en.backing_type.clone(),
                            cases: en.cases.iter().map(|c| c.name.clone()).collect(),
                        },
                    );
                }
                _ => {}
            }
        }

        // Pass 1: register type aliases and resolve interface signatures
        for item in &program.items {
            match item {
                Item::TypeAlias(ta) => {
                    self.type_aliases.insert(
                        ta.name.clone(),
                        TypeAliasInfo {
                            type_params: ta.type_params.clone(),
                            ty: ta.ty.clone(),
                        },
                    );
                }
                Item::Interface(iface) => {
                    let mut methods = HashMap::new();
                    for parent_name in &iface.extends {
                        if let Some(parent) = self.interfaces.get(parent_name) {
                            for (k, v) in &parent.methods {
                                methods.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    for m in &iface.methods {
                        let params = m
                            .params
                            .iter()
                            .map(|p| {
                                Ok((
                                    p.name.clone(),
                                    self.resolve_type(&p.ty, &[])?,
                                    p.default.is_some(),
                                ))
                            })
                            .collect::<Result<Vec<_>, CompileError>>()?;
                        let ret = self.resolve_type(&m.return_type, &[])?;
                        methods.insert(
                            m.name.clone(),
                            FnSig {
                                type_params: vec![],
                                params,
                                ret,
                            },
                        );
                    }
                    self.interfaces.insert(
                        iface.name.clone(),
                        InterfaceInfo {
                            extends: iface.extends.clone(),
                            methods,
                        },
                    );
                }
                _ => {}
            }
        }

        // Register classes & top-level functions (after type aliases/enums/interfaces)
        for item in &program.items {
            match item {
                Item::Class(cls) => {
                    self.register_class(cls)?;
                }
                Item::Function(f) => {
                    let params = f
                        .params
                        .iter()
                        .map(|p| {
                            Ok((
                                p.name.clone(),
                                self.resolve_type(&p.ty, &f.type_params)?,
                                p.default.is_some(),
                            ))
                        })
                        .collect::<Result<Vec<_>, CompileError>>()?;
                    let ret = self.resolve_type(&f.return_type, &f.type_params)?;
                    self.functions.insert(
                        f.name.clone(),
                        FnSig {
                            type_params: f.type_params.clone(),
                            params,
                            ret,
                        },
                    );
                }
                _ => {}
            }
        }

        // Pass 2: check bodies & top-level statements
        for item in &program.items {
            match item {
                Item::Class(cls) => self.check_class(cls)?,
                Item::Function(f) => self.check_fn(f)?,
                Item::Stmt(s) => self.check_stmt(s)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn register_class(&mut self, cls: &ClassDecl) -> Result<(), CompileError> {
        let mut fields = HashMap::new();
        for f in &cls.fields {
            let fty = self.resolve_type(&f.ty, &cls.type_params)?;
            fields.insert(f.name.clone(), (f.visibility, fty));
        }

        let constructor = if let Some(ctor) = &cls.constructor {
            let params = ctor
                .params
                .iter()
                .map(|p| {
                    Ok((
                        p.name.clone(),
                        self.resolve_type(&p.ty, &cls.type_params)?,
                        p.default.is_some(),
                    ))
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            Some(params)
        } else {
            None
        };

        let mut methods = HashMap::new();
        for m in &cls.methods {
            let mut all_tparams = cls.type_params.clone();
            all_tparams.extend(m.type_params.clone());
            let params = m
                .params
                .iter()
                .map(|p| {
                    Ok((
                        p.name.clone(),
                        self.resolve_type(&p.ty, &all_tparams)?,
                        p.default.is_some(),
                    ))
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            let ret = self.resolve_type(&m.return_type, &all_tparams)?;
            methods.insert(
                m.name.clone(),
                (
                    m.visibility,
                    FnSig {
                        type_params: m.type_params.clone(),
                        params,
                        ret,
                    },
                ),
            );
        }

        self.classes.insert(
            cls.name.clone(),
            ClassInfo {
                type_params: cls.type_params.clone(),
                extends: cls.extends.clone(),
                implements: cls.implements.clone(),
                fields,
                constructor,
                methods,
            },
        );
        Ok(())
    }

    fn check_class(&mut self, cls: &ClassDecl) -> Result<(), CompileError> {
        // Verify implemented interfaces
        for iface_name in &cls.implements {
            let Some(iface) = self.interfaces.get(iface_name).cloned() else {
                return Err(CompileError::type_err(
                    format!("Unknown interface '{}'", iface_name),
                    cls.span,
                ));
            };
            let class_info = self.classes.get(&cls.name).unwrap().clone();
            for (mname, isig) in &iface.methods {
                let Some((vis, msig)) = class_info.methods.get(mname) else {
                    return Err(CompileError::type_err(
                        format!(
                            "Class '{}' does not implement method '{}' from interface '{}'",
                            cls.name, mname, iface_name
                        ),
                        cls.span,
                    ));
                };
                if *vis != Visibility::Public {
                    return Err(CompileError::type_err(
                        format!("Interface method '{}' must be public", mname),
                        cls.span,
                    ));
                }
                if msig.params.len() != isig.params.len() || msig.ret != isig.ret {
                    return Err(CompileError::type_err(
                        format!(
                            "Method '{}' in '{}' does not match interface '{}'",
                            mname, cls.name, iface_name
                        ),
                        cls.span,
                    ));
                }
            }
        }

        let prev_class = self.current_class.replace(cls.name.clone());
        let this_args: Vec<Type> = cls
            .type_params
            .iter()
            .map(|tp| Type::TypeParam(tp.clone()))
            .collect();
        let this_ty = Type::Class(cls.name.clone(), this_args);

        if let Some(ctor) = &cls.constructor {
            self.push_scope();
            self.declare_var("this", this_ty.clone());
            for p in &ctor.params {
                let pty = self.resolve_type(&p.ty, &cls.type_params)?;
                self.declare_var(&p.name, pty);
            }
            self.current_return = Some(Type::Void);
            for s in &ctor.body {
                self.check_stmt(s)?;
            }
            self.current_return = None;
            self.pop_scope();
        }

        for m in &cls.methods {
            let mut all_tparams = cls.type_params.clone();
            all_tparams.extend(m.type_params.clone());
            self.push_scope();
            self.declare_var("this", this_ty.clone());
            for p in &m.params {
                let pty = self.resolve_type(&p.ty, &all_tparams)?;
                self.declare_var(&p.name, pty);
            }
            let ret = self.resolve_type(&m.return_type, &all_tparams)?;
            self.current_return = Some(ret);
            for s in &m.body {
                self.check_stmt(s)?;
            }
            if always_calls_method(&m.body, &m.name) {
                return Err(CompileError::type_err(
                    format!(
                        "Infinite recursion in method '{}::{}': '{}' is called unconditionally on every path without a base-case guard, which will exhaust the call stack and memory at runtime.",
                        cls.name, m.name, m.name
                    ),
                    m.span,
                ));
            }
            if contains_method_call(&m.body, &m.name) {
                self.warnings.push(CompileWarning::new(
                    format!(
                        "Recursive call in method '{}::{}': ensure recursion depth is bounded to avoid stack/memory exhaustion.",
                        cls.name, m.name
                    ),
                    m.span,
                ));
            }
            self.current_return = None;
            self.pop_scope();
        }

        self.current_class = prev_class;
        Ok(())
    }

    fn check_fn(&mut self, f: &FnDecl) -> Result<(), CompileError> {
        self.push_scope();
        for p in &f.params {
            let pty = self.resolve_type(&p.ty, &f.type_params)?;
            if let Some(def) = &p.default {
                let def_ty = self.check_expr(def)?;
                if !def_ty.is_assignable_to(&pty) {
                    return Err(CompileError::type_err(
                        format!(
                            "Default value of type '{}' is not assignable to parameter '${}: {}'",
                            def_ty, p.name, pty
                        ),
                        p.span,
                    ));
                }
            }
            self.declare_var(&p.name, pty);
        }
        let ret = self.resolve_type(&f.return_type, &f.type_params)?;
        self.current_return = Some(ret);
        for s in &f.body {
            self.check_stmt(s)?;
        }
        if always_calls_fn(&f.body, &f.name) {
            return Err(CompileError::type_err(
                format!(
                    "Infinite recursion in function '{}': '{}' is called unconditionally on every execution path without a base-case guard, which will exhaust the call stack and memory at runtime.",
                    f.name, f.name
                ),
                f.span,
            ));
        }
        if contains_fn_call(&f.body, &f.name) {
            self.warnings.push(CompileWarning::new(
                format!(
                    "Recursive call in function '{}': ensure recursion depth is bounded to prevent stack overflow / memory exhaustion.",
                    f.name
                ),
                f.span,
            ));
        }
        self.current_return = None;
        self.pop_scope();
        Ok(())
    }

    fn resolve_type(
        &self,
        ty: &TypeExpr,
        type_params: &[String],
    ) -> Result<Type, CompileError> {
        match ty {
            TypeExpr::Named(name, args, span) => {
                if type_params.contains(name) {
                    return Ok(Type::TypeParam(name.clone()));
                }
                match name.as_str() {
                    "int" => Ok(Type::Int),
                    "float" => Ok(Type::Float),
                    "string" => Ok(Type::String),
                    "bool" => Ok(Type::Bool),
                    "void" => Ok(Type::Void),
                    "never" => Ok(Type::Never),
                    "null" => Ok(Type::Null),
                    "mixed" => Ok(Type::Mixed),
                    "any" => Ok(Type::Any),
                    "array" => {
                        if args.len() == 1 {
                            Ok(Type::Array(Box::new(
                                self.resolve_type(&args[0], type_params)?,
                            )))
                        } else if args.is_empty() {
                            Ok(Type::Array(Box::new(Type::Mixed)))
                        } else {
                            Err(CompileError::type_err(
                                "'array' expects 0 or 1 type argument",
                                *span,
                            ))
                        }
                    }
                    "dict" => {
                        if args.len() != 2 {
                            return Err(CompileError::type_err(
                                "'dict<K, V>' expects 2 type arguments",
                                *span,
                            ));
                        }
                        Ok(Type::Dict(
                            Box::new(self.resolve_type(&args[0], type_params)?),
                            Box::new(self.resolve_type(&args[1], type_params)?),
                        ))
                    }
                    _ => {
                        if let Some(alias) = self.type_aliases.get(name) {
                            if self.resolving_aliases.borrow().contains(name) {
                                return Err(CompileError::type_err(
                                    format!(
                                        "Circular type alias '{}' detected: recursive type aliases cannot be resolved finitely and would cause infinite expansion.",
                                        name
                                    ),
                                    *span,
                                ));
                            }
                            if alias.type_params.len() != args.len() {
                                return Err(CompileError::type_err(
                                    format!(
                                        "Type alias '{}' expects {} type arguments, got {}",
                                        name,
                                        alias.type_params.len(),
                                        args.len()
                                    ),
                                    *span,
                                ));
                            }
                            let resolved_args = args
                                .iter()
                                .map(|a| self.resolve_type(a, type_params))
                                .collect::<Result<Vec<_>, _>>()?;
                            self.resolving_aliases.borrow_mut().insert(name.clone());
                            let base_res = self.resolve_type(&alias.ty, &alias.type_params);
                            self.resolving_aliases.borrow_mut().remove(name);
                            let base = base_res?;
                            let subst: HashMap<String, Type> = alias
                                .type_params
                                .iter()
                                .cloned()
                                .zip(resolved_args)
                                .collect();
                            return Ok(base.substitute(&subst));
                        }
                        if let Some(cls) = self.classes.get(name) {
                            if cls.type_params.len() != args.len() {
                                return Err(CompileError::type_err(
                                    format!(
                                        "Class '{}' expects {} type arguments, got {}",
                                        name,
                                        cls.type_params.len(),
                                        args.len()
                                    ),
                                    *span,
                                ));
                            }
                            let resolved_args = args
                                .iter()
                                .map(|a| self.resolve_type(a, type_params))
                                .collect::<Result<Vec<_>, _>>()?;
                            return Ok(Type::Class(name.clone(), resolved_args));
                        }
                        if self.interfaces.contains_key(name) {
                            return Ok(Type::Class(name.clone(), vec![]));
                        }
                        if self.enums.contains_key(name) {
                            return Ok(Type::Enum(name.clone()));
                        }
                        Err(CompileError::type_err(
                            format!("Unknown type '{}'", name),
                            *span,
                        ))
                    }
                }
            }
            TypeExpr::Array(elem, _) => {
                Ok(Type::Array(Box::new(self.resolve_type(elem, type_params)?)))
            }
            TypeExpr::Optional(inner, _) => {
                let inner_ty = self.resolve_type(inner, type_params)?;
                Ok(Type::union(vec![inner_ty, Type::Null]))
            }
            TypeExpr::Union(items, _) => {
                let resolved = items
                    .iter()
                    .map(|t| self.resolve_type(t, type_params))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Type::union(resolved))
            }
            TypeExpr::Shape(fields, _) => {
                let resolved = fields
                    .iter()
                    .map(|f| {
                        Ok((
                            f.name.clone(),
                            f.optional,
                            self.resolve_type(&f.ty, type_params)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, CompileError>>()?;
                Ok(Type::Shape(resolved))
            }
            TypeExpr::Function { params, ret, .. } => {
                let ps = params
                    .iter()
                    .map(|p| self.resolve_type(p, type_params))
                    .collect::<Result<Vec<_>, _>>()?;
                let r = self.resolve_type(ret, type_params)?;
                Ok(Type::Function {
                    params: ps,
                    ret: Box::new(r),
                })
            }
        }
    }

    fn is_assignable(&self, source: &Type, target: &Type) -> bool {
        if source.is_assignable_to(target) {
            return true;
        }
        if let (Type::Class(sn, _), Type::Class(tn, _)) = (source, target) {
            return self.is_subclass_or_implements(sn, tn);
        }
        false
    }

    fn is_subclass_or_implements(&self, sub: &str, sup: &str) -> bool {
        if sub == sup {
            return true;
        }
        if let Some(cls) = self.classes.get(sub) {
            if cls.implements.iter().any(|i| self.is_subclass_or_implements(i, sup)) {
                return true;
            }
            if let Some(parent) = &cls.extends {
                if self.is_subclass_or_implements(parent, sup) {
                    return true;
                }
            }
        }
        if let Some(iface) = self.interfaces.get(sub) {
            if iface.extends.iter().any(|p| self.is_subclass_or_implements(p, sup)) {
                return true;
            }
        }
        false
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::VarDecl {
                name,
                ty,
                init,
                span,
            } => {
                let init_ty = self.check_expr(init)?;
                let final_ty = if let Some(ty_expr) = ty {
                    let declared = self.resolve_type(ty_expr, &[])?;
                    if !self.is_assignable(&init_ty, &declared) {
                        return Err(CompileError::type_err(
                            format!(
                                "Cannot assign '{}' to variable '${}' of type '{}'",
                                init_ty, name, declared
                            ),
                            *span,
                        ));
                    }
                    declared
                } else {
                    init_ty
                };
                self.declare_var(name, final_ty);
                Ok(())
            }
            Stmt::ConstDecl {
                name,
                ty,
                init,
                span,
            } => {
                let init_ty = self.check_expr(init)?;
                let final_ty = if let Some(ty_expr) = ty {
                    let declared = self.resolve_type(ty_expr, &[])?;
                    if !self.is_assignable(&init_ty, &declared) {
                        return Err(CompileError::type_err(
                            format!(
                                "Cannot assign '{}' to constant '{}' of type '{}'",
                                init_ty, name, declared
                            ),
                            *span,
                        ));
                    }
                    declared
                } else {
                    init_ty
                };
                self.constants.insert(name.clone(), final_ty);
                Ok(())
            }
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let val_ty = self.check_expr(value)?;
                if let Expr::Var(name, _) = target {
                    if let Some(existing) = self.lookup_var(name) {
                        if !val_ty.is_assignable_to(&existing) {
                            return Err(CompileError::type_err(
                                format!(
                                    "Cannot assign '{}' to variable '${}' of type '{}'",
                                    val_ty, name, existing
                                ),
                                *span,
                            ));
                        }
                    } else {
                        // First assignment without type annotation infers type
                        self.declare_var(name, val_ty);
                    }
                    return Ok(());
                }
                let target_ty = self.check_expr(target)?;
                if !val_ty.is_assignable_to(&target_ty) {
                    return Err(CompileError::type_err(
                        format!("Cannot assign '{}' to '{}'", val_ty, target_ty),
                        *span,
                    ));
                }
                Ok(())
            }
            Stmt::Return { value, span } => {
                let ret_ty = match value {
                    Some(e) => self.check_expr(e)?,
                    None => Type::Void,
                };
                if let Some(expected) = &self.current_return {
                    if !self.is_assignable(&ret_ty, expected) {
                        return Err(CompileError::type_err(
                            format!(
                                "Return type '{}' is not assignable to expected '{}'",
                                ret_ty, expected
                            ),
                            *span,
                        ));
                    }
                }
                Ok(())
            }
            Stmt::Echo { value, .. } => {
                self.check_expr(value)?;
                Ok(())
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.check_expr(cond)?;
                if !cond_ty.is_assignable_to(&Type::Bool) && !matches!(cond_ty, Type::Mixed) {
                    // Allow truthy checks on nullable or bool, warn only if clearly void
                    if matches!(cond_ty, Type::Void) {
                        return Err(CompileError::type_err(
                            "Condition cannot be 'void'",
                            *span,
                        ));
                    }
                }

                let narrowing = self.extract_narrowing(cond);

                self.push_scope();
                if let Some((var_name, then_ty, _)) = &narrowing {
                    self.set_narrowed_var(var_name, then_ty.clone());
                }
                for s in then_branch {
                    self.check_stmt(s)?;
                }
                self.pop_scope();

                if let Some(else_stmts) = else_branch {
                    self.push_scope();
                    if let Some((var_name, _, else_ty)) = &narrowing {
                        self.set_narrowed_var(var_name, else_ty.clone());
                    }
                    for s in else_stmts {
                        self.check_stmt(s)?;
                    }
                    self.pop_scope();
                } else if always_returns(then_branch) {
                    if let Some((var_name, _, else_ty)) = &narrowing {
                        self.set_narrowed_var(var_name, else_ty.clone());
                    }
                }
                Ok(())
            }
            Stmt::While { cond, body, span } => {
                self.check_expr(cond)?;
                if matches!(cond, Expr::Bool(true, _)) && !contains_return(body) {
                    if contains_fn_call(body, "array_push") {
                        return Err(CompileError::type_err(
                            "Infinite 'while (true)' loop with 'array_push' and no 'return' exit path will exhaust memory (OOM) at runtime.",
                            *span,
                        ));
                    }
                    self.warnings.push(CompileWarning::new(
                        "Potential infinite loop: 'while (true)' has no 'return' statement and may hang at runtime.",
                        *span,
                    ));
                } elseif_array_push_warn(body, *span, &mut self.warnings);
                self.push_scope();
                for s in body {
                    self.check_stmt(s)?;
                }
                self.pop_scope();
                Ok(())
            }
            Stmt::ForEach {
                iterable,
                key_var,
                val_var,
                body,
                span,
            } => {
                let iter_ty = self.check_expr(iterable)?;
                let (k_ty, v_ty) = match iter_ty {
                    Type::Array(elem) => (Type::Int, *elem),
                    Type::Dict(k, v) => (*k, *v),
                    Type::Any | Type::Mixed => (Type::Mixed, Type::Mixed),
                    other => {
                        return Err(CompileError::type_err(
                            format!("Cannot iterate over non-array type '{}'", other),
                            *span,
                        ))
                    }
                };
                self.push_scope();
                if let Some(kv) = key_var {
                    self.declare_var(kv, k_ty);
                }
                self.declare_var(val_var, v_ty);
                for s in body {
                    self.check_stmt(s)?;
                }
                self.pop_scope();
                Ok(())
            }
            Stmt::ExprStmt { expr, .. } => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    fn extract_narrowing(&self, cond: &Expr) -> Option<(String, Type, Type)> {
        match cond {
            Expr::Unary {
                op: UnaryOp::Not,
                expr,
                ..
            } => {
                let (name, then_ty, else_ty) = self.extract_narrowing(expr)?;
                Some((name, else_ty, then_ty))
            }
            Expr::Binary {
                left,
                op: BinaryOp::NotEqEq,
                right,
                ..
            } => {
                if let (Expr::Var(name, _), Expr::Null(_)) | (Expr::Null(_), Expr::Var(name, _)) =
                    (left.as_ref(), right.as_ref())
                {
                    let cur = self.lookup_var(name)?;
                    return Some((name.clone(), cur.remove_null(), Type::Null));
                }
                None
            }
            Expr::Binary {
                left,
                op: BinaryOp::EqEqEq,
                right,
                ..
            } => {
                if let (Expr::Var(name, _), Expr::Null(_)) | (Expr::Null(_), Expr::Var(name, _)) =
                    (left.as_ref(), right.as_ref())
                {
                    let cur = self.lookup_var(name)?;
                    return Some((name.clone(), Type::Null, cur.remove_null()));
                }
                None
            }
            Expr::Call { callee, args, .. } if args.len() == 1 => {
                if let (Expr::Ident(fn_name, _), Expr::Var(var_name, _)) =
                    (callee.as_ref(), &args[0])
                {
                    let cur = self.lookup_var(var_name)?;
                    let target = match fn_name.as_str() {
                        "is_int" => Type::Int,
                        "is_string" => Type::String,
                        "is_bool" => Type::Bool,
                        "is_float" => Type::Float,
                        _ => return None,
                    };
                    let else_ty = remove_type_from_union(&cur, &target);
                    return Some((var_name.clone(), target, else_ty));
                }
                None
            }
            Expr::InstanceOf {
                expr, class_name, ..
            } => {
                if let Expr::Var(var_name, _) = expr.as_ref() {
                    let cur = self.lookup_var(var_name)?;
                    let target = Type::Class(class_name.clone(), vec![]);
                    let else_ty = remove_type_from_union(&cur, &target);
                    return Some((var_name.clone(), target, else_ty));
                }
                None
            }
            _ => None,
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Type, CompileError> {
        match expr {
            Expr::Int(_, _) => Ok(Type::Int),
            Expr::Float(_, _) => Ok(Type::Float),
            Expr::String(_, _) => Ok(Type::String),
            Expr::Bool(_, _) => Ok(Type::Bool),
            Expr::Null(_) => Ok(Type::Null),
            Expr::Var(name, span) => self.lookup_var(name).ok_or_else(|| {
                CompileError::type_err(format!("Undefined variable '${}'", name), *span)
            }),
            Expr::Ident(name, span) => {
                if let Some(ty) = self.constants.get(name) {
                    return Ok(ty.clone());
                }
                if let Some(sig) = self.functions.get(name) {
                    return Ok(Type::Function {
                        params: sig.params.iter().map(|(_, t, _)| t.clone()).collect(),
                        ret: Box::new(sig.ret.clone()),
                    });
                }
                Err(CompileError::type_err(
                    format!("Undefined identifier '{}'", name),
                    *span,
                ))
            }
            Expr::ArrayLit(items, _) => {
                if items.is_empty() {
                    return Ok(Type::Array(Box::new(Type::Any)));
                }
                let mut elem_types = Vec::new();
                for item in items {
                    elem_types.push(self.check_expr(item)?);
                }
                Ok(Type::Array(Box::new(Type::union(elem_types))))
            }
            Expr::AssocArrayLit(entries, _) => {
                let mut fields = Vec::new();
                for (k, v) in entries {
                    let vty = self.check_expr(v)?;
                    fields.push((k.clone(), false, vty));
                }
                Ok(Type::Shape(fields))
            }
            Expr::Unary { op, expr, span } => {
                let ty = self.check_expr(expr)?;
                match op {
                    UnaryOp::Neg => {
                        if !ty.is_numeric() {
                            return Err(CompileError::type_err(
                                format!("Cannot negate type '{}'", ty),
                                *span,
                            ));
                        }
                        Ok(ty)
                    }
                    UnaryOp::Not => Ok(Type::Bool),
                }
            }
            Expr::Cast { target_ty, expr, .. } => {
                self.check_expr(expr)?;
                Ok(match target_ty.as_str() {
                    "int" => Type::Int,
                    "float" => Type::Float,
                    "string" => Type::String,
                    "bool" => Type::Bool,
                    _ => Type::Mixed,
                })
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let lt = self.check_expr(left)?;
                let rt = self.check_expr(right)?;
                match op {
                    BinaryOp::Concat => {
                        if !lt.is_assignable_to(&Type::String) && !lt.is_numeric() {
                            return Err(CompileError::type_err(
                                format!("Left operand of '.' cannot be '{}'", lt),
                                *span,
                            ));
                        }
                        Ok(Type::String)
                    }
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                        if !lt.is_numeric() || !rt.is_numeric() {
                            return Err(CompileError::type_err(
                                format!("Arithmetic operands must be numeric, got '{}' and '{}'", lt, rt),
                                *span,
                            ));
                        }
                        if matches!(lt, Type::Float) || matches!(rt, Type::Float) {
                            Ok(Type::Float)
                        } else {
                            Ok(Type::Int)
                        }
                    }
                    BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
                    | BinaryOp::AndAnd
                    | BinaryOp::OrOr => Ok(Type::Bool),
                    BinaryOp::NullCoal => Ok(Type::union(vec![lt.remove_null(), rt])),
                }
            }
            Expr::InstanceOf { expr, class_name, span } => {
                self.check_expr(expr)?;
                if !self.classes.contains_key(class_name)
                    && !self.interfaces.contains_key(class_name)
                {
                    return Err(CompileError::type_err(
                        format!("Unknown class or interface '{}'", class_name),
                        *span,
                    ));
                }
                Ok(Type::Bool)
            }
            Expr::New {
                class_name,
                type_args,
                args,
                span,
            } => {
                let Some(cls) = self.classes.get(class_name).cloned() else {
                    return Err(CompileError::type_err(
                        format!("Unknown class '{}'", class_name),
                        *span,
                    ));
                };
                if cls.type_params.len() != type_args.len() {
                    return Err(CompileError::type_err(
                        format!(
                            "Class '{}' expects {} type arguments, got {}",
                            class_name,
                            cls.type_params.len(),
                            type_args.len()
                        ),
                        *span,
                    ));
                }
                let resolved_targs = type_args
                    .iter()
                    .map(|t| self.resolve_type(t, &[]))
                    .collect::<Result<Vec<_>, _>>()?;
                let subst: HashMap<String, Type> = cls
                    .type_params
                    .iter()
                    .cloned()
                    .zip(resolved_targs.clone())
                    .collect();

                if let Some(ctor_params) = &cls.constructor {
                    self.check_call_args(ctor_params, args, &subst, *span)?;
                } else if !args.is_empty() {
                    return Err(CompileError::type_err(
                        format!("Class '{}' constructor takes 0 arguments", class_name),
                        *span,
                    ));
                }

                Ok(Type::Class(class_name.clone(), resolved_targs))
            }
            Expr::Member {
                object,
                property,
                span,
            } => {
                let obj_ty = self.check_expr(object)?;
                if is_nullable(&obj_ty) {
                    return Err(CompileError::type_err(
                        format!(
                            "Cannot access property or method '{}' on nullable type '{}': value may be null at runtime, causing a PHP Error. Narrow with 'if ($x !== null)' first.",
                            property, obj_ty
                        ),
                        *span,
                    ));
                }
                if let Type::Enum(en_name) = &obj_ty {
                    if property == "value" {
                        if let Some(en) = self.enums.get(en_name) {
                            match en.backing_type.as_deref() {
                                Some("string") => return Ok(Type::String),
                                Some("int") => return Ok(Type::Int),
                                _ => {}
                            }
                        }
                    }
                    if property == "name" {
                        return Ok(Type::String);
                    }
                }
                if let Type::Class(cls_name, targs) = obj_ty {
                    let Some(cls) = self.classes.get(&cls_name).cloned() else {
                        return Err(CompileError::type_err(
                            format!("Unknown class '{}'", cls_name),
                            *span,
                        ));
                    };
                    let subst: HashMap<String, Type> =
                        cls.type_params.iter().cloned().zip(targs).collect();
                    if let Some((vis, fty)) = cls.fields.get(property) {
                        if *vis == Visibility::Private
                            && self.current_class.as_deref() != Some(&cls_name)
                        {
                            return Err(CompileError::type_err(
                                format!("Cannot access private property '${}'", property),
                                *span,
                            ));
                        }
                        return Ok(fty.substitute(&subst));
                    }
                    if let Some((_, msig)) = cls.methods.get(property) {
                        return Ok(Type::Function {
                            params: msig
                                .params
                                .iter()
                                .map(|(_, t, _)| t.substitute(&subst))
                                .collect(),
                            ret: Box::new(msig.ret.substitute(&subst)),
                        });
                    }
                    return Err(CompileError::type_err(
                        format!(
                            "Class '{}' has no property or method '{}': accessing an undefined member causes a runtime Error in PHP.",
                            cls_name, property
                        ),
                        *span,
                    ));
                }
                Err(CompileError::type_err(
                    format!(
                        "Cannot access member '{}' on non-object type '{}': only class instances and backed enums support '->' member access.",
                        property, obj_ty
                    ),
                    *span,
                ))
            }
            Expr::StaticAccess {
                class_name,
                member,
                span,
            } => {
                if class_name == "PDO" {
                    return match member.as_str() {
                        "ATTR_ERRMODE" | "ERRMODE_EXCEPTION" | "FETCH_ASSOC" => Ok(Type::Int),
                        _ => Err(CompileError::type_err(
                            format!("Unknown PDO constant '{}'", member),
                            *span,
                        )),
                    };
                }
                if let Some(en) = self.enums.get(class_name) {
                    if en.cases.contains(member) {
                        return Ok(Type::Enum(class_name.clone()));
                    }
                    return Err(CompileError::type_err(
                        format!("Enum '{}' has no case '{}'", class_name, member),
                        *span,
                    ));
                }
                Err(CompileError::type_err(
                    format!("Unknown enum or class '{}'", class_name),
                    *span,
                ))
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj_ty = self.check_expr(object)?;
                let idx_ty = self.check_expr(index)?;
                if is_nullable(&obj_ty) {
                    return Err(CompileError::type_err(
                        format!(
                            "Cannot index into nullable type '{}': dereferencing null causes a runtime error in PHP. Narrow with 'if ($x !== null)' or use '??' first.",
                            obj_ty
                        ),
                        *span,
                    ));
                }
                match obj_ty {
                    Type::Array(elem) => {
                        if !idx_ty.is_assignable_to(&Type::Int) {
                            return Err(CompileError::type_err(
                                format!("Array index must be 'int', got '{}'", idx_ty),
                                *span,
                            ));
                        }
                        Ok(*elem)
                    }
                    Type::Dict(k, v) => {
                        if !idx_ty.is_assignable_to(&k) {
                            return Err(CompileError::type_err(
                                format!("Dict key must be '{}', got '{}'", k, idx_ty),
                                *span,
                            ));
                        }
                        Ok(*v)
                    }
                    Type::Shape(fields) => {
                        if let Expr::String(key, _) = index.as_ref() {
                            if let Some((_, opt, fty)) = fields.iter().find(|(n, _, _)| n == key) {
                                if *opt {
                                    self.warnings.push(CompileWarning::new(
                                        format!(
                                            "Optional shape key '{}' may not exist at runtime; prefer '??' or 'array_key_exists(\"{}\", ...)' to avoid 'Undefined array key' warnings.",
                                            key, key
                                        ),
                                        *span,
                                    ));
                                }
                                return Ok(if *opt {
                                    Type::union(vec![fty.clone(), Type::Null])
                                } else {
                                    fty.clone()
                                });
                            }
                            let known: Vec<&str> = fields.iter().map(|(n, _, _)| n.as_str()).collect();
                            return Err(CompileError::type_err(
                                format!(
                                    "Shape has no key '{}' (available keys: [{}]): accessing a non-existent key causes an 'Undefined array key' warning in PHP 8.4.",
                                    key,
                                    known.join(", ")
                                ),
                                *span,
                            ));
                        }
                        Ok(Type::Mixed)
                    }
                    Type::Any | Type::Mixed => Ok(Type::Mixed),
                    other => Err(CompileError::type_err(
                        format!("Cannot index into type '{}'", other),
                        *span,
                    )),
                }
            }
            Expr::Call {
                callee,
                type_args,
                args,
                span,
            } => {
                if let Expr::Ident(fn_name, _) = callee.as_ref() {
                    if let Some(sig) = self.functions.get(fn_name).cloned() {
                        return self.check_generic_call(&sig, type_args, args, *span);
                    }
                }
                if let Expr::Member {
                    object,
                    property,
                    span: mspan,
                } = callee.as_ref()
                {
                    let obj_ty = self.check_expr(object)?;
                    if let Type::Class(cls_name, class_targs) = obj_ty {
                        if let Some(iface) = self.interfaces.get(&cls_name).cloned() {
                            let Some(msig) = iface.methods.get(property).cloned() else {
                                return Err(CompileError::type_err(
                                    format!(
                                        "Interface '{}' has no method '{}'",
                                        cls_name, property
                                    ),
                                    *mspan,
                                ));
                            };
                            return self.check_generic_call(&msig, type_args, args, *span);
                        }
                        let Some(cls) = self.classes.get(&cls_name).cloned() else {
                            return Err(CompileError::type_err(
                                format!("Unknown class '{}'", cls_name),
                                *mspan,
                            ));
                        };
                        let Some((vis, msig)) = cls.methods.get(property) else {
                            return Err(CompileError::type_err(
                                format!("Class '{}' has no method '{}'", cls_name, property),
                                *mspan,
                            ));
                        };
                        if *vis == Visibility::Private
                            && self.current_class.as_deref() != Some(&cls_name)
                        {
                            return Err(CompileError::type_err(
                                format!("Cannot call private method '{}'", property),
                                *mspan,
                            ));
                        }
                        let mut class_subst: HashMap<String, Type> =
                            cls.type_params.iter().cloned().zip(class_targs).collect();
                        let mut instantiated = msig.clone();
                        instantiated.params = instantiated
                            .params
                            .iter()
                            .map(|(n, t, d)| (n.clone(), t.substitute(&class_subst), *d))
                            .collect();
                        instantiated.ret = instantiated.ret.substitute(&class_subst);
                        let _ = &mut class_subst;
                        return self.check_generic_call(&instantiated, type_args, args, *span);
                    }
                }

                let callee_ty = self.check_expr(callee)?;
                match callee_ty {
                    Type::Function { params, ret } => {
                        if args.len() != params.len() {
                            return Err(CompileError::type_err(
                                format!(
                                    "Expected {} arguments, got {}",
                                    params.len(),
                                    args.len()
                                ),
                                *span,
                            ));
                        }
                        for (arg, pty) in args.iter().zip(params.iter()) {
                            let aty = self.check_expr(arg)?;
                            if !self.is_assignable(&aty, pty) {
                                return Err(CompileError::type_err(
                                    format!(
                                        "Argument of type '{}' is not assignable to '{}'",
                                        aty, pty
                                    ),
                                    arg.span(),
                                ));
                            }
                        }
                        Ok(*ret)
                    }
                    Type::Any => Ok(Type::Any),
                    other => Err(CompileError::type_err(
                        format!("Type '{}' is not callable", other),
                        *span,
                    )),
                }
            }
            Expr::ArrowFn {
                params,
                return_type,
                body,
                span,
            } => {
                self.push_scope();
                let mut param_types = Vec::new();
                for p in params {
                    let pty = self.resolve_type(&p.ty, &[])?;
                    self.declare_var(&p.name, pty.clone());
                    param_types.push(pty);
                }
                let body_ty = self.check_expr(body)?;
                self.pop_scope();

                let ret_ty = if let Some(rt) = return_type {
                    let declared = self.resolve_type(rt, &[])?;
                    if !body_ty.is_assignable_to(&declared) {
                        return Err(CompileError::type_err(
                            format!(
                                "Arrow function returns '{}' which is not assignable to '{}'",
                                body_ty, declared
                            ),
                            *span,
                        ));
                    }
                    declared
                } else {
                    body_ty
                };

                Ok(Type::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                })
            }
        }
    }

    fn check_generic_call(
        &mut self,
        sig: &FnSig,
        type_args: &[TypeExpr],
        args: &[Expr],
        span: Span,
    ) -> Result<Type, CompileError> {
        let mut subst = HashMap::new();
        if !type_args.is_empty() {
            if type_args.len() != sig.type_params.len() {
                return Err(CompileError::type_err(
                    format!(
                        "Expected {} type arguments, got {}",
                        sig.type_params.len(),
                        type_args.len()
                    ),
                    span,
                ));
            }
            for (tp, te) in sig.type_params.iter().zip(type_args.iter()) {
                subst.insert(tp.clone(), self.resolve_type(te, &[])?);
            }
        } else if !sig.type_params.is_empty() {
            // Simple type argument inference from call arguments
            for ((_, pty, _), arg) in sig.params.iter().zip(args.iter()) {
                let aty = self.check_expr(arg)?;
                infer_type_params(pty, &aty, &mut subst);
            }
        }

        self.check_call_args(&sig.params, args, &subst, span)?;
        Ok(sig.ret.substitute(&subst))
    }

    fn check_call_args(
        &mut self,
        params: &[(String, Type, bool)],
        args: &[Expr],
        subst: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(), CompileError> {
        let min_args = params.iter().filter(|(_, _, has_def)| !has_def).count();
        if args.len() < min_args || args.len() > params.len() {
            return Err(CompileError::type_err(
                format!(
                    "Expected {}..={} arguments, got {}",
                    min_args,
                    params.len(),
                    args.len()
                ),
                span,
            ));
        }
        for (arg, (pname, pty, _)) in args.iter().zip(params.iter()) {
            let aty = self.check_expr(arg)?;
            let expected = pty.substitute(subst);
            if !self.is_assignable(&aty, &expected) {
                return Err(CompileError::type_err(
                    format!(
                        "Argument for '${}' has type '{}', expected '{}'",
                        pname, aty, expected
                    ),
                    arg.span(),
                ));
            }
        }
        Ok(())
    }
}

fn infer_type_params(param_ty: &Type, arg_ty: &Type, subst: &mut HashMap<String, Type>) {
    match (param_ty, arg_ty) {
        (Type::TypeParam(name), _) => {
            subst.entry(name.clone()).or_insert_with(|| arg_ty.clone());
        }
        (Type::Array(pe), Type::Array(ae)) => infer_type_params(pe, ae, subst),
        (Type::Dict(pk, pv), Type::Dict(ak, av)) => {
            infer_type_params(pk, ak, subst);
            infer_type_params(pv, av, subst);
        }
        (Type::Class(pn, pa), Type::Class(an, aa)) if pn == an && pa.len() == aa.len() => {
            for (p, a) in pa.iter().zip(aa.iter()) {
                infer_type_params(p, a, subst);
            }
        }
        _ => {}
    }
}

fn remove_type_from_union(source: &Type, to_remove: &Type) -> Type {
    match source {
        Type::Union(items) => {
            let remaining: Vec<Type> = items
                .iter()
                .filter(|t| *t != to_remove)
                .cloned()
                .collect();
            Type::union(remaining)
        }
        other if other == to_remove => Type::Never,
        other => other.clone(),
    }
}

fn always_returns(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Return { .. } => true,
        Stmt::If {
            then_branch,
            else_branch: Some(else_stmts),
            ..
        } => always_returns(then_branch) && always_returns(else_stmts),
        _ => false,
    })
}

fn is_nullable(ty: &Type) -> bool {
    match ty {
        Type::Null => true,
        Type::Union(items) => items.iter().any(|t| matches!(t, Type::Null)),
        _ => false,
    }
}

fn contains_return(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Return { .. } => true,
        Stmt::If {
            then_branch,
            else_branch,
            ..
        } => {
            contains_return(then_branch)
                || else_branch.as_ref().is_some_and(|e| contains_return(e))
        }
        Stmt::While { body, .. } | Stmt::ForEach { body, .. } => contains_return(body),
        _ => false,
    })
}

fn elseif_array_push_warn(body: &[Stmt], span: Span, warnings: &mut Vec<CompileWarning>) {
    if contains_fn_call(body, "array_push") {
        warnings.push(CompileWarning::new(
            "Memory growth warning: 'array_push' inside a 'while' loop can cause unbounded memory allocation if loop bounds are not strictly controlled.",
            span,
        ));
    }
}

fn always_calls_fn(stmts: &[Stmt], fn_name: &str) -> bool {
    for s in stmts {
        if always_returns(std::slice::from_ref(s)) && !stmt_always_calls_fn(s, fn_name) {
            return false;
        }
        if let Stmt::If {
            then_branch,
            else_branch: None,
            ..
        } = s
        {
            if always_returns(then_branch) && !always_calls_fn(then_branch, fn_name) {
                return false;
            }
        }
        if stmt_always_calls_fn(s, fn_name) {
            return true;
        }
    }
    false
}

fn stmt_always_calls_fn(stmt: &Stmt, fn_name: &str) -> bool {
    match stmt {
        Stmt::VarDecl { init, .. }
        | Stmt::ConstDecl { init, .. }
        | Stmt::Assign { value: init, .. }
        | Stmt::Echo { value: init, .. }
        | Stmt::ExprStmt { expr: init, .. } => expr_contains_fn_call(init, fn_name),
        Stmt::Return { value: Some(v), .. } => expr_contains_fn_call(v, fn_name),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_contains_fn_call(cond, fn_name)
                || (always_calls_fn(then_branch, fn_name)
                    && else_branch
                        .as_ref()
                        .is_some_and(|eb| always_calls_fn(eb, fn_name)))
        }
        _ => false,
    }
}

fn always_calls_method(stmts: &[Stmt], method_name: &str) -> bool {
    for s in stmts {
        if always_returns(std::slice::from_ref(s)) && !stmt_always_calls_method(s, method_name) {
            return false;
        }
        if let Stmt::If {
            then_branch,
            else_branch: None,
            ..
        } = s
        {
            if always_returns(then_branch) && !always_calls_method(then_branch, method_name) {
                return false;
            }
        }
        if stmt_always_calls_method(s, method_name) {
            return true;
        }
    }
    false
}

fn stmt_always_calls_method(stmt: &Stmt, method_name: &str) -> bool {
    match stmt {
        Stmt::VarDecl { init, .. }
        | Stmt::ConstDecl { init, .. }
        | Stmt::Assign { value: init, .. }
        | Stmt::Echo { value: init, .. }
        | Stmt::ExprStmt { expr: init, .. } => expr_contains_method_call(init, method_name),
        Stmt::Return { value: Some(v), .. } => expr_contains_method_call(v, method_name),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_contains_method_call(cond, method_name)
                || (always_calls_method(then_branch, method_name)
                    && else_branch
                        .as_ref()
                        .is_some_and(|eb| always_calls_method(eb, method_name)))
        }
        _ => false,
    }
}

fn contains_fn_call(stmts: &[Stmt], fn_name: &str) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::VarDecl { init, .. }
        | Stmt::ConstDecl { init, .. }
        | Stmt::Assign { value: init, .. }
        | Stmt::Echo { value: init, .. }
        | Stmt::ExprStmt { expr: init, .. } => expr_contains_fn_call(init, fn_name),
        Stmt::Return { value: Some(v), .. } => expr_contains_fn_call(v, fn_name),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_contains_fn_call(cond, fn_name)
                || contains_fn_call(then_branch, fn_name)
                || else_branch
                    .as_ref()
                    .is_some_and(|eb| contains_fn_call(eb, fn_name))
        }
        Stmt::While { cond, body, .. } | Stmt::ForEach { iterable: cond, body, .. } => {
            expr_contains_fn_call(cond, fn_name) || contains_fn_call(body, fn_name)
        }
        _ => false,
    })
}

fn expr_contains_fn_call(expr: &Expr, fn_name: &str) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Expr::Ident(n, _) = callee.as_ref() {
                if n == fn_name {
                    return true;
                }
            }
            expr_contains_fn_call(callee, fn_name)
                || args.iter().any(|a| expr_contains_fn_call(a, fn_name))
        }
        Expr::Binary { left, right, .. } => {
            expr_contains_fn_call(left, fn_name) || expr_contains_fn_call(right, fn_name)
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::InstanceOf { expr, .. } => {
            expr_contains_fn_call(expr, fn_name)
        }
        Expr::ArrayLit(items, _) | Expr::New { args: items, .. } => {
            items.iter().any(|a| expr_contains_fn_call(a, fn_name))
        }
        Expr::AssocArrayLit(entries, _) => entries
            .iter()
            .any(|(_, v)| expr_contains_fn_call(v, fn_name)),
        Expr::Member { object, .. } => expr_contains_fn_call(object, fn_name),
        Expr::Index { object, index, .. } => {
            expr_contains_fn_call(object, fn_name) || expr_contains_fn_call(index, fn_name)
        }
        _ => false,
    }
}

fn contains_method_call(stmts: &[Stmt], method_name: &str) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::VarDecl { init, .. }
        | Stmt::ConstDecl { init, .. }
        | Stmt::Assign { value: init, .. }
        | Stmt::Echo { value: init, .. }
        | Stmt::ExprStmt { expr: init, .. } => expr_contains_method_call(init, method_name),
        Stmt::Return { value: Some(v), .. } => expr_contains_method_call(v, method_name),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_contains_method_call(cond, method_name)
                || contains_method_call(then_branch, method_name)
                || else_branch
                    .as_ref()
                    .is_some_and(|eb| contains_method_call(eb, method_name))
        }
        Stmt::While { cond, body, .. } | Stmt::ForEach { iterable: cond, body, .. } => {
            expr_contains_method_call(cond, method_name) || contains_method_call(body, method_name)
        }
        _ => false,
    })
}

fn expr_contains_method_call(expr: &Expr, method_name: &str) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Expr::Member {
                object, property, ..
            } = callee.as_ref()
            {
                if matches!(object.as_ref(), Expr::Var(v, _) if v == "this")
                    && property == method_name
                {
                    return true;
                }
            }
            expr_contains_method_call(callee, method_name)
                || args.iter().any(|a| expr_contains_method_call(a, method_name))
        }
        Expr::Binary { left, right, .. } => {
            expr_contains_method_call(left, method_name)
                || expr_contains_method_call(right, method_name)
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::InstanceOf { expr, .. } => {
            expr_contains_method_call(expr, method_name)
        }
        Expr::ArrayLit(items, _) | Expr::New { args: items, .. } => {
            items.iter().any(|a| expr_contains_method_call(a, method_name))
        }
        Expr::AssocArrayLit(entries, _) => entries
            .iter()
            .any(|(_, v)| expr_contains_method_call(v, method_name)),
        Expr::Member { object, .. } => expr_contains_method_call(object, method_name),
        Expr::Index { object, index, .. } => {
            expr_contains_method_call(object, method_name)
                || expr_contains_method_call(index, method_name)
        }
        _ => false,
    }
}
