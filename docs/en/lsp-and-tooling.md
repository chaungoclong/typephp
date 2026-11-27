# Language Server (LSP), VS Code Extension & Tooling

> [🇻🇳 Tiếng Việt](../lsp-and-tooling.md) | 🇺🇸 English

This document explains how the **TypePHP Language Server (`tphpc --lsp`)**, the **VS Code Extension (`editors/vscode-typephp/`)**, and the installation/dev tooling work.

---

## 1. TypePHP Language Server (`src/lsp.rs`)

The LSP server is built directly into `tphpc` via the `--lsp` flag (`src/main.rs`) using pure Rust `std` (zero external crates).

> **Note**: `src/lsp.rs` only reads `Lexer`, `Parser`, and `check_with_warnings` for editor features; it never writes `.php` files or alters compiler output.

### 1.1. Protocol & Lifecycle

- Communicates over `stdin`/`stdout` using standard `Content-Length: <bytes>\r\n\r\n<json>` framing and a self-contained JSON parser/serializer (`Json`, `parse_json`, `to_string_compact`).
- Supports `initialize`, `textDocument/didOpen`, `textDocument/didChange`, `textDocument/hover`, `textDocument/definition`, `textDocument/references`, `textDocument/publishDiagnostics`, `shutdown`, and `exit`.

### 1.2. 2-Pass AST Indexing (`SymbolIndex::build`)

1. **Pass 1 (Declarations)**: Registers `import`, `type`, `interface` (with methods), `enum` (with cases and backing values), `class` (with fields and methods), and `function` using `find_ident_span_after` to point directly at symbol identifiers.
2. **Pass 2 (Bodies & Local Type Tracking)**: Walks `Stmt`, `Expr`, and `TypeExpr` with lexical scopes (`scopes` for variable/parameter IDs and `var_types` for class-type inference such as `$db = new PDO(...)` → `$stmt = $db->prepare(...)` → `PDOStatement`). Indexes `const` and `enum case` initializers with `format_expr_preview` so hover displays constant values.

### 1.3. PHP Reflection Catalog (`PhpCatalog` + `OnceLock`)

- On first built-in lookup, `PhpCatalog::load_via_php_cli()` runs `php -r` **once** (cached in `std::sync::OnceLock<PhpCatalog>`) to inspect:
  - `get_defined_functions()["internal"]` via `ReflectionFunction`
  - `get_declared_classes()` / `get_declared_interfaces()` via `ReflectionClass` / `ReflectionMethod` (including PHP 8.x `getTentativeReturnType()` and `getConstants()` values)
  - `get_defined_constants(true)` for global PHP constants (`PHP_EOL`, `JSON_THROW_ON_ERROR`, etc.)
- Falls back to `PhpCatalog::fallback()` if `php` is not available in `PATH`.

### 1.4. Hover, `Ctrl + Click` (Definition) & References

- **Hover**: Shows symbol kind, signature or constant value, definition line (or `PHP built-in / external symbol`), and reference count.
- **`Ctrl + Click` / `F12`**:
  - On a usage → jumps to its declaration (including cross-file `import { ... } from "..."`).
  - On a declaration or PHP built-in symbol → returns `Location[]` of all references in the file so VS Code opens Peek References or jumps to usages.
- **Find All References (`Shift + F12`)**: Returns every `SymbolOcc` sharing the same `def_id`.

---

## 2. VS Code Extension (`editors/vscode-typephp/`)

Pure Node.js extension (no `npm install` required):

- [package.json](../../editors/vscode-typephp/package.json): registers the `typephp` language (`.tphp`), `onLanguage:typephp` activation, and `typephp.serverPath` (default `"tphpc"`).
- [extension.js](../../editors/vscode-typephp/extension.js): spawns `serverPath --lsp` and registers Hover, Definition, Reference, and Diagnostic providers.
- [syntaxes/typephp.tmLanguage.json](../../editors/vscode-typephp/syntaxes/typephp.tmLanguage.json): TextMate grammar for PHP + TypePHP syntax.

---

## 3. Installers & Docker Dev Environment

- **`install/install.sh` & `install/install.ps1`**: Cross-OS installers for Linux, macOS, and Windows (auto-installs PHP, Composer, PHPStan Level 9, and `tphpc`).
- **`Makefile`, `Dockerfile`, `compose.yaml`**: Containerized Rust + PHP 8.4 + Composer + PHPStan dev environment.
