# Language Design & Compiler Architecture (`tphpc`)

> [🇻🇳 Tiếng Việt](../architecture.md) | 🇺🇸 English

This document describes the design philosophy of **TypePHP** and the internal architecture of `tphpc` (written in pure Rust with `std` only, zero external crates).

---

## 1. Design Philosophy

1. **Familiar PHP Syntax**: Keep PHP syntax (`$var`, `function`, `->`, `::`, `.`, `foreach`, `elseif`, `fn (...) => ...`, `global`) so PHP developers can adopt it immediately and reuse the PHP ecosystem (PDO, PSR-7, Composer).
2. **Strong Static Typing**:
   - Typed local variables (`$count: int = 0;`, `$items: int[] = [1, 2, 3];`, `$map: dict<string, int> = [];`) and type inference.
   - Type aliases & structural shapes (`type UserShape = { id: int, name: string, email?: string };`).
   - Generics (`function identity<T>(T $x): T`, `class Box<T>`) with argument type inference.
   - Control-flow narrowing in `if`/`else` and after early returns.
3. **Safety, PSR-12 & Zero Runtime Overhead**:
   - All `type` aliases, shapes, and generic parameters are erased during codegen.
   - Emitted PHP uses `declare(strict_types=1);`, follows **PSR-12**, and includes PHPDoc annotations to pass **PHPStan Level 9**.

---

## 2. Compilation Pipeline

```text
Source (.tphp)
   │
   ▼  load_all_modules() (DFS topological order via import/require_once)
[1] Lexer        (src/lexer.rs)   ──► Vec<Token>
   │
   ▼
[2] Parser       (src/parser.rs)  ──► Program (AST in src/ast.rs)
   │
   ▼
[3] TypeChecker  (src/checker.rs) ──► Type checking, generics, narrowing (src/types.rs)
   │
   ▼
[4] Codegen      (src/codegen.rs) ──► PSR-12 PHP 8.x + PHPDoc + Tree-Shaking + main()
   │
   ▼
[5] PHPStan L9   (src/lib.rs)     ──► Mandatory verification gate (level=9)
   │
   ▼  (If --minify is enabled)
[6] Minifier     (src/minify.rs + minify_php) ──► Var/fn mangling + PHP/HTML/CSS/JS minification
```

---

## 3. Module Breakdown (`src/`)

### `src/error.rs` — Spans & Diagnostics
- **`Span { line, col }`**: 1-based line and column tracking.
- **`CompileError { stage, message, span }`**: distinguishes `LexError`, `ParseError`, and `TypeError`.

### `src/lexer.rs` — Tokenizer (`Lexer`)
- Converts `.tphp` source into `Vec<Token>`, skipping `<?php`, `?>`, whitespace, and comments (`//`, `#`, `/* ... */`).
- Recognizes `$var`, PHP/TypePHP keywords (`function`, `fn`, `class`, `interface`, `enum`, `case`, `type`, `import`, `from`, `namespace`, `use`, `global`, `echo`, ...), and operators.

### `src/ast.rs` — Abstract Syntax Tree
- **`Program`**: list of `Item`s (`Namespace`, `Use`, `Require`, `Import`, `TypeAlias`, `Interface`, `Enum`, `Class`, `Function`, `Stmt`).
- **`TypeExpr`**: `Named`, `Array`, `Union`, `Optional`, `Shape`, `Function`.
- **`Stmt` & `Expr`**: statements (`VarDecl`, `ConstDecl`, `Assign`, `Return`, `Echo`, `Global`, `If`, `While`, `ForEach`, `ExprStmt`) and expressions.

### `src/parser.rs` — Parser
- Recursive-descent parser with operator-precedence climbing and generic-call lookahead (`is_generic_call_ahead`).

### `src/types.rs` — Semantic Type Model (`Type`)
- `Int`, `Float`, `String`, `Bool`, `Void`, `Never`, `Null`, `Mixed`, `Any`, `Array`, `Dict`, `Union`, `Shape`, `Function`, `Class`, `Enum`, `TypeParam`.
- `Type::union`, `Type::substitute`, `Type::is_assignable_to`.

### `src/checker.rs` — 3-Pass `TypeChecker`
1. **Pass 0**: Pre-registers `Interface` and `Enum` names for forward/cross references.
2. **Pass 1**: Registers `TypeAlias`, `Interface` method signatures, `Class` members, and top-level `Function`s.
3. **Pass 2**: Type-checks class/function bodies and top-level statements, including `implements` verification, generic inference, control-flow narrowing, and PHP built-ins (superglobals `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `$_SESSION`, `$_FILES`, `$_REQUEST`, `$_ENV`, `$GLOBALS`, `PDO`, `PDOStatement`, common stdlib functions).

### `src/codegen.rs` — PSR-12 Code Generator (`Codegen`)
- **`emit_program_with_aliases_and_entry`** (`--mode preserve`): emits individual `.php` files with `declare(strict_types=1);` and PSR-12 formatting.
- **`emit_bundle`** (`--mode bundle`): merges modules into a single `.php` file, wraps `namespace { ... }` blocks, strips internal `.tphp` `require_once`s, and runs **`compute_reachable_symbols`** (BFS tree-shaking from `main()` / top-level statements) to drop unused declarations, `use` statements, and empty namespaces.
- **`entry_auto_main_call`**: emits `main();` or `exit(main());` (including `string[] $args` and `int $argc, string[] $argv`) at the end of entrypoints defining `function main(...)`.
- **`format_phpstan_type` & `emit_fn_phpdoc`**: generates `@var`, `@param`, `@return`, `@template` PHPDoc for PHPStan Level 9.

### `src/minify.rs` — Mangling & Inline Web Minifier
- **`mangle_programs`**: shortens local variables (`$a`, `$b`, ...) and top-level user functions (`_a`, `_b`, ...) in bundle mode while preserving `$this`, superglobals, `global` variables, and `main`.
- **`minify_html_css_js`**: minifies inline HTML, `<style>` CSS, and `<script>` JS inside string literals.

### `src/lib.rs` & `src/main.rs` — Module Loader, CLI & PHPStan Gate
- **`load_all_modules`**: DFS topological loader with cycle protection.
- **`run_phpstan_level9`**: runs `phpstan analyse --level=9 --no-progress` before allowing compilation to succeed.

### `src/lsp.rs` — Built-in Language Server (`tphpc --lsp`)
- Runs only when `--lsp` is passed; never affects `.php` compilation output.
- See [lsp-and-tooling.md](lsp-and-tooling.md) for full details.
