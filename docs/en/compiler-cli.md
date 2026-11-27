# Compiler `tphpc` & CLI Options

> [🇻🇳 Tiếng Việt](../compiler-cli.md) | 🇺🇸 English

The `tphpc` compiler is written in pure Rust (`std` only) and executes:
**Lexer → Parser → TypeChecker → Codegen (PSR-12 + PHPDoc + Tree-Shaking) → PHPStan Level 9 Gate → (optional) Minifier**.

---

## Command-Line Usage

```bash
tphpc <input.tphp> [OPTIONS]
```

| Option | Description | Default |
| :--- | :--- | :--- |
| `-o, --out <path>` | Output file (with `--mode bundle`) or output root directory (with `--mode preserve`) | Same directory as `.tphp` |
| `--target <8.1\|8.2\|8.3\|8.4>` | Target PHP version | `8.4` |
| `--mode <preserve\|bundle>` | `preserve`: keep directory layout; `bundle`: single `.php` file + tree-shaking | `preserve` |
| `--bundle` | Shorthand for `--mode bundle` | `false` |
| `--minify` | Minify PHP, mangle local vars/fns, and minify inline HTML/CSS/JS | `false` |
| `--check` | Type-check only without emitting `.php` files | `false` |
| `--lsp` | Start TypePHP Language Server (LSP over stdio) | `false` |

---

## Examples

### 1. Compile preserving directory structure into `dist/`

```bash
cargo run -- examples/blog_app/public/index.tphp --mode preserve -o dist
```

### 2. Bundle + tree-shake + target PHP 8.2 + minify

```bash
cargo run -- examples/blog_app/public/index.tphp --target 8.2 --bundle --minify -o dist/blog.min.php
```

---

## PHPStan Level 9 Gate

Every build (except `--check`) automatically runs:

```bash
phpstan analyse --level=9 --no-progress <files...>
```

- If PHPStan Level 9 reports any error, `tphpc` fails with exit code `1`.
- When `--minify` is enabled, PHPStan Level 9 runs on the full PHPDoc output **before** mangling and stripping comments.
