use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Never,
    Null,
    Mixed,
    Any,
    Array(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Union(Vec<Type>),
    Shape(Vec<(String, bool, Type)>), // (name, optional, type)
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Class(String, Vec<Type>),
    Enum(String),
    TypeParam(String),
}

impl Type {
    pub fn union(types: Vec<Type>) -> Type {
        let mut flat = Vec::new();
        for t in types {
            match t {
                Type::Union(inner) => flat.extend(inner),
                Type::Never => {}
                other => {
                    if !flat.contains(&other) {
                        flat.push(other);
                    }
                }
            }
        }
        if flat.is_empty() {
            Type::Never
        } else if flat.len() == 1 {
            flat.into_iter().next().unwrap()
        } else {
            Type::Union(flat)
        }
    }

    pub fn substitute(&self, subst: &HashMap<String, Type>) -> Type {
        match self {
            Type::TypeParam(name) => subst.get(name).cloned().unwrap_or_else(|| self.clone()),
            Type::Array(elem) => Type::Array(Box::new(elem.substitute(subst))),
            Type::Dict(k, v) => {
                Type::Dict(Box::new(k.substitute(subst)), Box::new(v.substitute(subst)))
            }
            Type::Union(items) => {
                Type::union(items.iter().map(|t| t.substitute(subst)).collect())
            }
            Type::Shape(fields) => Type::Shape(
                fields
                    .iter()
                    .map(|(n, opt, t)| (n.clone(), *opt, t.substitute(subst)))
                    .collect(),
            ),
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| p.substitute(subst)).collect(),
                ret: Box::new(ret.substitute(subst)),
            },
            Type::Class(name, args) => Type::Class(
                name.clone(),
                args.iter().map(|a| a.substitute(subst)).collect(),
            ),
            _ => self.clone(),
        }
    }

    pub fn remove_null(&self) -> Type {
        match self {
            Type::Null => Type::Never,
            Type::Union(items) => {
                let remaining: Vec<Type> = items
                    .iter()
                    .filter(|t| !matches!(t, Type::Null))
                    .cloned()
                    .collect();
                Type::union(remaining)
            }
            other => other.clone(),
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Any)
    }

    /// Check if `source` is assignable to `target`.
    pub fn is_assignable_to(&self, target: &Type) -> bool {
        if matches!(target, Type::Any | Type::Mixed) || matches!(self, Type::Any | Type::Never) {
            return true;
        }
        if self == target {
            return true;
        }
        // int is assignable to float in PHP strict_types? Actually in PHP strict_types=1,
        // int is accepted for float parameters!
        if matches!(self, Type::Int) && matches!(target, Type::Float) {
            return true;
        }
        // Union target: source must be assignable to at least one member
        if let Type::Union(targets) = target {
            if let Type::Union(sources) = self {
                return sources.iter().all(|s| s.is_assignable_to(target));
            }
            return targets.iter().any(|t| self.is_assignable_to(t));
        }
        // Union source: every member must be assignable to target
        if let Type::Union(sources) = self {
            return sources.iter().all(|s| s.is_assignable_to(target));
        }
        match (self, target) {
            (Type::Array(s_elem), Type::Array(t_elem)) => s_elem.is_assignable_to(t_elem),
            (Type::Array(s_elem), Type::Dict(_, _)) if matches!(s_elem.as_ref(), Type::Any) => true,
            (Type::Dict(sk, sv), Type::Dict(tk, tv)) => {
                sk.is_assignable_to(tk) && sv.is_assignable_to(tv)
            }
            (Type::Shape(s_fields), Type::Shape(t_fields)) => {
                for (t_name, t_opt, t_ty) in t_fields {
                    match s_fields.iter().find(|(sn, _, _)| sn == t_name) {
                        Some((_, s_opt, s_ty)) => {
                            if *s_opt && !*t_opt {
                                return false;
                            }
                            if !s_ty.is_assignable_to(t_ty) {
                                return false;
                            }
                        }
                        None => {
                            if !*t_opt {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            (Type::Shape(s_fields), Type::Dict(tk, tv)) => {
                matches!(tk.as_ref(), Type::String | Type::Any | Type::Mixed)
                    && s_fields.iter().all(|(_, _, ty)| ty.is_assignable_to(tv))
            }
            (
                Type::Function {
                    params: sp,
                    ret: sr,
                },
                Type::Function {
                    params: tp,
                    ret: tr,
                },
            ) => {
                sp.len() == tp.len()
                    && tp
                        .iter()
                        .zip(sp.iter())
                        .all(|(t_p, s_p)| t_p.is_assignable_to(s_p))
                    && sr.is_assignable_to(tr)
            }
            (Type::Class(sn, sa), Type::Class(tn, ta)) => {
                sn == tn
                    && sa.len() == ta.len()
                    && sa.iter().zip(ta.iter()).all(|(a, b)| a == b)
            }
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Void => write!(f, "void"),
            Type::Never => write!(f, "never"),
            Type::Null => write!(f, "null"),
            Type::Mixed => write!(f, "mixed"),
            Type::Any => write!(f, "any"),
            Type::Array(elem) => write!(f, "{}[]", elem),
            Type::Dict(k, v) => write!(f, "dict<{}, {}>", k, v),
            Type::Union(items) => {
                let parts: Vec<String> = items.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", parts.join("|"))
            }
            Type::Shape(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(n, opt, t)| {
                        if *opt {
                            format!("{}?: {}", n, t)
                        } else {
                            format!("{}: {}", n, t)
                        }
                    })
                    .collect();
                write!(f, "{{ {} }}", parts.join(", "))
            }
            Type::Function { params, ret } => {
                let ps: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({}) => {}", ps.join(", "), ret)
            }
            Type::Class(name, args) => {
                if args.is_empty() {
                    write!(f, "{}", name)
                } else {
                    let as_: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    write!(f, "{}<{}>", name, as_.join(", "))
                }
            }
            Type::Enum(name) => write!(f, "{}", name),
            Type::TypeParam(name) => write!(f, "{}", name),
        }
    }
}
