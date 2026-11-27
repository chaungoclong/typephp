# TypePHP (`tphpc`) Documentation

> [🇻🇳 Tiếng Việt](../README.md) | 🇺🇸 English

**TypePHP** is a statically typed superset of PHP (inspired by TypeScript) that keeps familiar **PHP syntax** and compiles to clean, **PSR-12**-compliant **PHP 8.1–8.4** (`declare(strict_types=1);`) verified by **PHPStan Level 9**.

## Table of Contents

1. [Language Syntax & Type System (`language-syntax.md`)](language-syntax.md)
2. [Language Design & Compiler Architecture (`architecture.md`)](architecture.md)
3. [Compiler `tphpc` & CLI Options (`compiler-cli.md`)](compiler-cli.md)
4. [PSR-7 Micro-Framework & Validation (`psr7-microfw.md`)](psr7-microfw.md)
5. [Language Server (LSP), VS Code Extension & Tooling (`lsp-and-tooling.md`)](lsp-and-tooling.md)

---

## Quick Start

### 1. Run the compiler test suite (Rust)

```bash
cargo test
```

### 2. Compile a file or multi-file project

```bash
# Compile preserving directory structure + PHPStan Level 9
cargo run -- examples/blog_app/public/index.tphp

# Bundle into a single file + tree-shaking + minify (PHP, mangling, HTML/CSS/JS)
cargo run -- examples/blog_app/public/index.tphp --bundle --minify -o dist/app.php
```

### 3. Included Examples

- [examples/demo.tphp](../../examples/demo.tphp) — Generics, shapes, enums, union narrowing, `main()`.
- [examples/tree_shake_demo/](../../examples/tree_shake_demo/main.tphp) — 4-level transitive tree-shaking demo (`main -> lib1 -> lib2 -> lib3`).
- [examples/rest_api/](../../examples/rest_api/README.md) — SQLite REST API example.
- [examples/psr7_microfw/](../../examples/psr7_microfw/README.md) — Composer PSR-4 micro-framework + Validator.
- [examples/blog_app/](../../examples/blog_app/README.md) — Blog + Admin app (Tailwind CSS, SQLite, Cookie Auth).
