# AGENTS.md — TypePHP (`tphpc`)

This repository contains **TypePHP**, a statically typed superset of PHP with a compiler (`tphpc`) written in **pure Rust (`std` only, zero external crates)** targeting **PHP 8.1–8.4**.

## Repository Layout

- `src/` — Rust compiler (`tphpc`):
  - `error.rs` — `Span` and `CompileError` (`LexError`, `ParseError`, `TypeError`)
  - `lexer.rs` — Tokenizer (`$var`, PHP/TypePHP keywords, operators)
  - `ast.rs` — AST nodes (`Program`, `Item`, `TypeExpr`, `Stmt`, `Expr`)
  - `parser.rs` — Recursive-descent + precedence-climbing parser
  - `types.rs` — Semantic `Type` model, generic `substitute`, `is_assignable_to`
  - `checker.rs` — 3-pass `TypeChecker`, generic inference, control-flow narrowing, PHP builtins
  - `codegen.rs` — PSR-12 PHP emitter, PHPStan PHPDoc generator, `use` tree-shaking, `minify_php`
  - `minify.rs` — Local variable/function mangling and inline HTML/CSS/JS minifier
  - `lsp.rs` — Built-in LSP server (`tphpc --lsp`) with `SymbolIndex` and PHP Reflection catalog
  - `lib.rs` — Module graph loader (`load_all_modules`), `compile_with_options`, PHPStan Level 9 gate
  - `main.rs` — CLI (`tphpc`)
- `tests/compiler_tests.rs` — Integration tests for the compiler
- `docs/` — Language, architecture, CLI, and framework documentation
- `examples/psr7_microfw/` — Reusable Composer PSR-4 micro-framework (`Http\Message`, `Routing\App`, `Validation`) + `vendor/bin/phpstan`
- `examples/blog_app/` — Standalone Blog + Admin app (Tailwind CSS, SQLite, cookie auth) consuming `psr7_microfw`
- `examples/rest_api/` — Standalone SQLite REST API example

## Build, Test & Verify Commands

Always run these after modifying compiler code (`src/`) or `.tphp` examples:

```bash
# 1. Run Rust unit & integration tests (must have 0 warnings)
cargo test

# 2. Compile & run PHPStan Level 9 on the PSR-7 micro-framework + Blog App
cargo run -- examples/blog_app/public/index.tphp

# 3. Verify bundle + minify mode + PHP syntax
cargo run -- examples/blog_app/public/index.tphp --bundle --minify -o /tmp/blog.min.php && php -l /tmp/blog.min.php
```

## Engineering Conventions

1. **Zero external Rust dependencies**: Keep `Cargo.toml` free of external crates (`std` only).
2. **Mandatory PHPStan Level 9 & PSR-12**:
   - `compile_with_options` enforces `phpstan analyse --level=9` before succeeding.
   - Emitted PHP must follow PSR-12 (blank line after `<?php`, `declare(strict_types=1);`, Allman braces for classes/interfaces/enums/functions/methods, `fn (...)` spacing).
   - If `--minify` is enabled, always run PHPStan Level 9 on the full PHPDoc output *before* mangling and stripping.
3. **Framework vs. App Separation**:
   - Keep `examples/psr7_microfw/` strictly framework-only (PSR-7, Router, Validator).
   - Place applications (like `examples/blog_app/`) in their own directories importing from `psr7_microfw`.
4. **Adding New Language Features / Builtins**:
   - Update `lexer.rs` → `ast.rs` → `parser.rs` → `checker.rs` → `codegen.rs` (and `minify.rs` if new AST variants are added).
   - Add integration tests in `tests/compiler_tests.rs` and update `docs/` if user-facing syntax or CLI flags change.
5. **Cleanup**: Do not leave temporary test artifacts (`/tmp/*.php`, `*.db` created during ad-hoc verification) tracked or cluttered in the workspace.

## Git & Pull Request Workflow

The `main` branch is **protected** — never push directly to `main`.

1. **Always branch off `main`**: Create a new descriptive branch before committing (`feat/<topic>`, `fix/<topic>`, `docs/<topic>`, etc.):
   ```bash
   git checkout -b feat/my-change
   ```
2. **Test before commit**: Run `cargo test` (0 warnings) and `cargo run -- examples/blog_app/public/index.tphp` (PHPStan Level 9 OK).
3. **Commit & Push**: Stage only relevant files (`git add`), commit with a clear message, push the branch (`git push -u origin feat/my-change`), and open a Pull Request into `main`.
