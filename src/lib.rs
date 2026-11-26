pub mod ast;
pub mod checker;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod minify;
pub mod parser;
pub mod types;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ast::{Item, Program, TypeExpr};
use checker::TypeChecker;
use codegen::{minify_php, Codegen, EmitMode, PhpVersion};
use error::{CompileError, Span};
use lexer::Lexer;
use parser::Parser;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub target: PhpVersion,
    pub mode: EmitMode,
    pub output: Option<PathBuf>,
    pub minify: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: PhpVersion::Php84,
            mode: EmitMode::Preserve,
            output: None,
            minify: false,
        }
    }
}

pub fn check(source: &str) -> Result<(), CompileError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    let mut checker = TypeChecker::new();
    checker.check_program(&program)?;
    Ok(())
}

pub fn compile(source: &str) -> Result<String, CompileError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    let mut checker = TypeChecker::new();
    checker.check_program(&program)?;
    let mut codegen = Codegen::new();
    Ok(codegen.emit_program(&program))
}

pub fn check_file(entry: &Path) -> Result<(), CompileError> {
    let modules = load_all_modules(entry)?;
    let mut checker = TypeChecker::new();
    for (_, program) in &modules {
        checker.check_program(program)?;
    }
    Ok(())
}

pub fn compile_file(
    entry: &Path,
    output: Option<&Path>,
) -> Result<Vec<(PathBuf, PathBuf)>, CompileError> {
    let opts = CompileOptions {
        target: PhpVersion::Php84,
        mode: EmitMode::Preserve,
        output: output.map(PathBuf::from),
        minify: false,
    };
    compile_with_options(entry, &opts)
}

pub fn compile_with_options(
    entry: &Path,
    opts: &CompileOptions,
) -> Result<Vec<(PathBuf, PathBuf)>, CompileError> {
    let modules = load_all_modules(entry)?;
    let mut checker = TypeChecker::new();
    for (_, program) in &modules {
        checker.check_program(program)?;
    }

    let mut compiled = Vec::new();
    let entry_canon = fs::canonicalize(entry).unwrap_or_else(|_| entry.to_path_buf());

    match opts.mode {
        EmitMode::Bundle => {
            let mut codegen = Codegen::new_with_target(opts.target);
            let refs: Vec<&Program> = modules.iter().map(|(_, p)| p).collect();
            let php = codegen.emit_bundle(&refs);

            let out_path = match &opts.output {
                Some(p) if p.is_dir() => {
                    let name = entry_canon
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("bundle.php"));
                    p.join(name).with_extension("php")
                }
                Some(p) => p.clone(),
                None => entry_canon.with_extension("php"),
            };

            if let Some(parent) = out_path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|e| {
                        CompileError::parse(
                            format!("Failed to create directory '{}': {}", parent.display(), e),
                            Span::new(1, 1),
                        )
                    })?;
                }
            }

            fs::write(&out_path, php).map_err(|e| {
                CompileError::parse(
                    format!("Failed to write '{}': {}", out_path.display(), e),
                    Span::new(1, 1),
                )
            })?;
            compiled.push((entry_canon, out_path));
        }
        EmitMode::Preserve => {
            let mut external_aliases: HashMap<String, TypeExpr> = HashMap::new();
            let mod_paths: Vec<PathBuf> = modules.iter().map(|(p, _)| p.clone()).collect();
            let base_root = common_ancestor(&mod_paths);
            let last_idx = modules.len().saturating_sub(1);

            for (idx, (mod_path, program)) in modules.iter().enumerate() {
                let mut codegen = Codegen::new_with_target(opts.target);
                let php = codegen.emit_program_with_aliases(program, &external_aliases);
                for item in &program.items {
                    if let Item::TypeAlias(ta) = item {
                        external_aliases.insert(ta.name.clone(), ta.ty.clone());
                    }
                }

                let out_path = match &opts.output {
                    Some(out_base) => {
                        if modules.len() == 1 && out_base.extension().is_some_and(|e| e == "php") {
                            out_base.clone()
                        } else {
                            let rel = mod_path.strip_prefix(&base_root).unwrap_or(mod_path);
                            out_base.join(rel).with_extension("php")
                        }
                    }
                    None => {
                        let _ = idx == last_idx;
                        mod_path.with_extension("php")
                    }
                };

                if let Some(parent) = out_path.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent).map_err(|e| {
                            CompileError::parse(
                                format!("Failed to create directory '{}': {}", parent.display(), e),
                                Span::new(1, 1),
                            )
                        })?;
                    }
                }

                fs::write(&out_path, php).map_err(|e| {
                    CompileError::parse(
                        format!("Failed to write '{}': {}", out_path.display(), e),
                        Span::new(1, 1),
                    )
                })?;
                compiled.push((mod_path.clone(), out_path));
            }
        }
    }

    let out_files: Vec<PathBuf> = compiled.iter().map(|(_, dst)| dst.clone()).collect();
    run_phpstan_level9(entry, &out_files)?;

    if opts.minify {
        let mut mangled_programs: Vec<Program> = modules.iter().map(|(_, p)| p.clone()).collect();
        minify::mangle_programs(&mut mangled_programs, opts.mode == EmitMode::Bundle);

        match opts.mode {
            EmitMode::Bundle => {
                let mut codegen = Codegen::new_with_target(opts.target);
                let refs: Vec<&Program> = mangled_programs.iter().collect();
                let php = minify_php(&codegen.emit_bundle(&refs));
                fs::write(&out_files[0], php).map_err(|e| {
                    CompileError::parse(
                        format!("Failed to write minified '{}': {}", out_files[0].display(), e),
                        Span::new(1, 1),
                    )
                })?;
            }
            EmitMode::Preserve => {
                let mut external_aliases: HashMap<String, TypeExpr> = HashMap::new();
                for (idx, program) in mangled_programs.iter().enumerate() {
                    let mut codegen = Codegen::new_with_target(opts.target);
                    let php = minify_php(&codegen.emit_program_with_aliases(program, &external_aliases));
                    for item in &program.items {
                        if let Item::TypeAlias(ta) = item {
                            external_aliases.insert(ta.name.clone(), ta.ty.clone());
                        }
                    }
                    fs::write(&out_files[idx], php).map_err(|e| {
                        CompileError::parse(
                            format!("Failed to write minified '{}': {}", out_files[idx].display(), e),
                            Span::new(1, 1),
                        )
                    })?;
                }
            }
        }
    }

    Ok(compiled)
}

fn common_ancestor(paths: &[PathBuf]) -> PathBuf {
    let Some(first) = paths.first() else {
        return PathBuf::from(".");
    };
    let mut prefix = first.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    for p in paths.iter().skip(1) {
        while !p.starts_with(&prefix) {
            if !prefix.pop() {
                return PathBuf::from("/");
            }
        }
    }
    prefix
}

fn find_phpstan(entry: &Path) -> Option<PathBuf> {
    let mut cur = fs::canonicalize(entry)
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    while let Some(dir) = cur {
        let candidate = dir.join("vendor/bin/phpstan");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent().map(Path::to_path_buf);
    }
    let cwd_candidate = PathBuf::from("vendor/bin/phpstan");
    if cwd_candidate.is_file() {
        return Some(cwd_candidate);
    }
    let fw_candidate = PathBuf::from("examples/psr7_microfw/vendor/bin/phpstan");
    if fw_candidate.is_file() {
        return Some(fw_candidate);
    }
    if Command::new("phpstan")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return Some(PathBuf::from("phpstan"));
    }
    None
}

fn run_phpstan_level9(entry: &Path, php_files: &[PathBuf]) -> Result<(), CompileError> {
    let Some(phpstan) = find_phpstan(entry) else {
        return Err(CompileError::type_err(
            "PHPStan binary not found (install phpstan/phpstan via composer)",
            Span::new(1, 1),
        ));
    };

    let mut cmd = Command::new(&phpstan);
    cmd.arg("analyse")
        .arg("--level=9")
        .arg("--no-progress");
    for f in php_files {
        cmd.arg(f);
    }

    let output = cmd.output().map_err(|e| {
        CompileError::type_err(
            format!("Failed to execute PHPStan '{}': {}", phpstan.display(), e),
            Span::new(1, 1),
        )
    })?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CompileError::type_err(
            format!("PHPStan level 9 failed:\n{}{}", stdout, stderr),
            Span::new(1, 1),
        ));
    }

    Ok(())
}

fn load_all_modules(entry: &Path) -> Result<Vec<(PathBuf, Program)>, CompileError> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    load_module_dfs(entry, &mut visited, &mut order)?;
    Ok(order)
}

fn load_module_dfs(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    order: &mut Vec<(PathBuf, Program)>,
) -> Result<(), CompileError> {
    let canon = fs::canonicalize(path).map_err(|e| {
        CompileError::parse(
            format!("Cannot resolve module '{}': {}", path.display(), e),
            Span::new(1, 1),
        )
    })?;
    if visited.contains(&canon) {
        return Ok(());
    }
    visited.insert(canon.clone());

    let source = fs::read_to_string(&canon).map_err(|e| {
        CompileError::parse(
            format!("Failed to read '{}': {}", canon.display(), e),
            Span::new(1, 1),
        )
    })?;
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    let base_dir = canon.parent().unwrap_or_else(|| Path::new("."));
    for item in &program.items {
        let dep_str = match item {
            Item::Import { path, .. } | Item::Require(path, _) if path.ends_with(".tphp") => {
                Some(path)
            }
            _ => None,
        };
        if let Some(rel) = dep_str {
            let dep_path = base_dir.join(rel);
            load_module_dfs(&dep_path, visited, order)?;
        }
    }

    order.push((canon, program));
    Ok(())
}
