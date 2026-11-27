<p align="center">
  <img src="docs/assets/banner.svg" alt="TypePHP Banner" width="100%" />
</p>

<p align="center">
  <img src="docs/assets/logo.svg" alt="TypePHP Logo" width="110" />
</p>

<h1 align="center">TypePHP (<code>tphpc</code>)</h1>

<p align="center">
  <a href="#tiếng-việt">🇻🇳 Tiếng Việt</a> · <a href="#english">🇺🇸 English</a>
</p>

---

<a id="tiếng-việt"></a>
## 🇻🇳 Tiếng Việt

**TypePHP** là ngôn ngữ lập trình kiểu tĩnh mạnh (lấy cảm hứng từ TypeScript) giữ nguyên phong cách cú pháp **PHP** và biên dịch sang mã **PHP 8.1–8.4** (`declare(strict_types=1);`) an toàn, tối ưu hiệu suất, chuẩn **PSR-12** và vượt qua **PHPStan Level 9**.

Trình biên dịch `tphpc` (kèm Language Server `--lsp`) được viết hoàn toàn bằng **Rust thuần (`std` only, không phụ thuộc crate ngoài)**.

### Tính năng chính

- **Cú pháp PHP quen thuộc + Định kiểu mạnh**: khai báo kiểu cho biến (`$x: int = 10;`), suy luận kiểu tự động, `const`, `function`, `class`, `interface`, backed `enum`, arrow function `fn (...) => ...`, `global $var;`, và đầy đủ PHP superglobals (`$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `$_SESSION`, `$_FILES`, `$_REQUEST`, `$_ENV`, `$GLOBALS`).
- **Generics & Type Aliases**: hỗ trợ generic trên hàm, class, type alias (`identity<T>`, `Box<T>`, `Pair<A, B>`) kèm suy luận tham số kiểu.
- **Structural Shapes**: `type UserShape = { id: int, name: string, email?: string };` kiểm tra key/kiểu lúc compile và biên dịch thành `array` PHP với zero runtime overhead.
- **Union, Nullable & Control-Flow Narrowing**: `int|string|null`, `?string`, tự động thu hẹp kiểu trong nhánh `if`, `else` và sau `return` sớm (`$x === null`, `is_int($x)`, `!is_string($x)`, `$x instanceof Foo`).
- **Module đa file & Tree-Shaking (`--bundle`)**: `import { ... } from './...'`, `namespace`, `use` với đồ thị phụ thuộc; ở chế độ `--bundle`, compiler duyệt BFS từ entrypoint (`main()` hoặc top-level statements) để loại bỏ toàn bộ class, interface, enum, function, `use` và namespace không dùng tới.
- **Entrypoint `function main()` tuỳ chọn (kiểu Go / C)**: hỗ trợ `main(): void`, `main(): int`, `main(string[] $args)`, `main(int $argc, string[] $argv)` — tự động gọi `main();` hoặc `exit(main());` ở cuối entrypoint.
- **PHPStan Level 9 Gate**: tự động sinh PHPDoc (`@var`, `@param`, `@return`, `@template`) và chạy `phpstan analyse --level=9` ngay trong bước biên dịch.
- **LSP Server & VS Code Extension**: tích hợp `tphpc --lsp` (Hover chữ ký & giá trị hằng số, `Ctrl+Click` Go-to-Definition / References, PHP Reflection catalog tự động quét 1.700+ hàm & 280+ class PHP gốc, Live Diagnostics) cùng extension thuần Node tại [editors/vscode-typephp/](editors/vscode-typephp/).
- **Tuỳ chọn đóng gói & tối ưu**: `--target <8.1|8.2|8.3|8.4>`, `--mode <preserve|bundle>`, `-o/--out <path>`, `--check`, và `--minify` (nén PHP, rút gọn tên biến/hàm, nén HTML/CSS/JS nhúng trong chuỗi).

### Ví dụ nhanh

**`hello.tphp`**:
```php
<?php

type UserShape = {
    id: int,
    name: string,
    email?: string,
};

function formatId(int|string|null $id): string {
    if ($id === null) {
        return 'none';
    }
    if (is_int($id)) {
        return 'ID#' . (string) $id;
    }
    return $id;
}

function main(): void {
    $user: UserShape = ['id' => 1, 'name' => 'Long'];
    echo formatId($user['id']) . ' - ' . $user['name'];
}
```

### Cài đặt & Sử dụng CLI

**Linux & macOS** (`x86_64` / `arm64`):
```bash
curl -fsSL https://raw.githubusercontent.com/chaungoclong/typephp/main/install/install.sh | bash
```

**Windows (PowerShell)**:
```powershell
irm https://raw.githubusercontent.com/chaungoclong/typephp/main/install/install.ps1 | iex
```

**Build từ source (Cargo)**:
```bash
cargo install --path .
```

**Môi trường Docker dev**:
```bash
docker compose run --rm dev cargo test
docker compose run --rm dev bash
```

**Biên dịch & Kiểm thử**:
```bash
# Chạy toàn bộ test suite
cargo test

# Kiểm tra kiểu tĩnh (không sinh file .php)
cargo run -- examples/blog_app/public/index.tphp --check

# Biên dịch giữ nguyên cấu trúc thư mục + chạy PHPStan Level 9
cargo run -- examples/blog_app/public/index.tphp

# Biên dịch gộp 1 file + tree-shaking + minify
cargo run -- examples/blog_app/public/index.tphp --bundle --minify -o dist/blog.min.php
```

### Tài liệu tiếng Việt

- [Tổng quan tài liệu (`docs/README.md`)](docs/README.md)
- [Cú pháp & Hệ thống kiểu (`docs/language-syntax.md`)](docs/language-syntax.md)
- [Kiến trúc Compiler (`docs/architecture.md`)](docs/architecture.md)
- [Trình biên dịch & CLI (`docs/compiler-cli.md`)](docs/compiler-cli.md)
- [LSP Server, VS Code Extension & Tooling (`docs/lsp-and-tooling.md`)](docs/lsp-and-tooling.md)
- [PSR-7 Micro-Framework (`docs/psr7-microfw.md`)](docs/psr7-microfw.md)

---

<a id="english"></a>
## 🇺🇸 English

**TypePHP** is a statically typed superset of PHP (inspired by TypeScript) that preserves familiar **PHP syntax** and compiles to clean, **PSR-12**-compliant **PHP 8.1–8.4** (`declare(strict_types=1);`) verified by **PHPStan Level 9**.

The `tphpc` compiler (including its built-in `--lsp` Language Server) is written in **pure Rust (`std` only, zero external crates)**.

### Key Features

- **Familiar PHP Syntax + Static Typing**: typed variables (`$x: int = 10;`), type inference, `const`, `function`, `class`, `interface`, backed `enum`, arrow functions `fn (...) => ...`, `global $var;`, and full PHP superglobals (`$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `$_SESSION`, `$_FILES`, `$_REQUEST`, `$_ENV`, `$GLOBALS`).
- **Generics & Type Aliases**: generic functions, classes, and type aliases (`identity<T>`, `Box<T>`, `Pair<A, B>`) with type argument inference.
- **Structural Shapes**: `type UserShape = { id: int, name: string, email?: string };` checked at compile time and emitted as plain PHP `array` with zero runtime overhead.
- **Unions, Nullables & Control-Flow Narrowing**: `int|string|null`, `?string`, automatic narrowing inside `if`/`else` and after early returns (`$x === null`, `is_int($x)`, `!is_string($x)`, `$x instanceof Foo`).
- **Multi-file Modules & Bundle Tree-Shaking (`--bundle`)**: `import { ... } from './...'`, `namespace`, `use` with DFS dependency loading; in `--bundle` mode, BFS reachability from the entrypoint (`main()` or top-level statements) strips unused classes, interfaces, enums, functions, `use` imports, and empty namespaces.
- **Optional Go/C-style `function main()` Entrypoint**: supports `main(): void`, `main(): int`, `main(string[] $args)`, and `main(int $argc, string[] $argv)` — automatically emitting `main();` or `exit(main());`.
- **Mandatory PHPStan Level 9 Gate**: generates `@var`, `@param`, `@return`, `@template` PHPDoc and enforces `phpstan analyse --level=9` on every build.
- **Built-in LSP Server & VS Code Extension**: `tphpc --lsp` provides Hover (signatures & constant values), `Ctrl+Click` Go-to-Definition / References, automatic PHP Reflection catalog (1,700+ native functions & 280+ classes/interfaces), and Live Diagnostics via [editors/vscode-typephp/](editors/vscode-typephp/).
- **Packaging & Optimization**: `--target <8.1|8.2|8.3|8.4>`, `--mode <preserve|bundle>`, `-o/--out <path>`, `--check`, and `--minify` (token minification, variable/function mangling, inline HTML/CSS/JS minification).

### Quick Example

**`hello.tphp`**:
```php
<?php

type UserShape = {
    id: int,
    name: string,
    email?: string,
};

function formatId(int|string|null $id): string {
    if ($id === null) {
        return 'none';
    }
    if (is_int($id)) {
        return 'ID#' . (string) $id;
    }
    return $id;
}

function main(): void {
    $user: UserShape = ['id' => 1, 'name' => 'Long'];
    echo formatId($user['id']) . ' - ' . $user['name'];
}
```

### Installation & CLI Usage

**Linux & macOS** (`x86_64` / `arm64`):
```bash
curl -fsSL https://raw.githubusercontent.com/chaungoclong/typephp/main/install/install.sh | bash
```

**Windows (PowerShell)**:
```powershell
irm https://raw.githubusercontent.com/chaungoclong/typephp/main/install/install.ps1 | iex
```

**Build from source (Cargo)**:
```bash
cargo install --path .
```

**Docker Dev Environment**:
```bash
docker compose run --rm dev cargo test
docker compose run --rm dev bash
```

**Compile & Test**:
```bash
# Run full test suite
cargo test

# Type-check only without emitting PHP
cargo run -- examples/blog_app/public/index.tphp --check

# Compile preserving directory structure + PHPStan Level 9
cargo run -- examples/blog_app/public/index.tphp

# Bundle + tree-shake + minify into a single PHP file
cargo run -- examples/blog_app/public/index.tphp --bundle --minify -o dist/blog.min.php
```

### English Documentation

- [Documentation Home (`docs/en/README.md`)](docs/en/README.md)
- [Language & Type System (`docs/en/language-syntax.md`)](docs/en/language-syntax.md)
- [Compiler Architecture (`docs/en/architecture.md`)](docs/en/architecture.md)
- [Compiler CLI (`docs/en/compiler-cli.md`)](docs/en/compiler-cli.md)
- [LSP Server, VS Code Extension & Tooling (`docs/en/lsp-and-tooling.md`)](docs/en/lsp-and-tooling.md)
- [PSR-7 Micro-Framework (`docs/en/psr7-microfw.md`)](docs/en/psr7-microfw.md)

---

## Repository Layout / Cấu trúc thư mục

```text
typephp/
├── src/                        # Pure-Rust compiler & LSP server (std only)
│   ├── lexer.rs, ast.rs, parser.rs, types.rs, checker.rs, codegen.rs, minify.rs, lsp.rs, lib.rs, main.rs
├── tests/                      # 8 test suites (lexer, parser, ast, types_checker, codegen, minify, lsp, e2e)
├── docs/                       # Vietnamese docs + docs/en/ English docs
├── editors/vscode-typephp/     # Pure-Node VS Code extension (LSP client + TextMate grammar)
├── install/                    # Cross-OS installers (install.sh, install.ps1)
└── examples/
    ├── demo.tphp               # Core language syntax demo
    ├── tree_shake_demo/        # 4-level transitive tree-shaking demo (main -> lib1 -> lib2 -> lib3)
    ├── rest_api/               # SQLite REST API example
    ├── docker_rest_api/        # Minimal Dockerized REST API example
    ├── psr7_microfw/           # Composer PSR-4 micro-framework (PSR-7 + Router + Validator)
    └── blog_app/               # Blog + Admin app (Tailwind CSS, SQLite, Cookie Auth)
```

## References / Tài liệu tham khảo

- [Language Specification (`SPEC.md`)](SPEC.md)
- [Developer Guide & PR Rules (`AGENTS.md`)](AGENTS.md)
