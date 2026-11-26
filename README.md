# TypePHP (`tphpc`)

**TypePHP** là ngôn ngữ lập trình kiểu tĩnh mạnh (lấy cảm hứng từ TypeScript) giữ nguyên phong cách cú pháp **PHP** và biên dịch sang mã **PHP 8.1–8.4** (`declare(strict_types=1);`) an toàn, tối ưu hiệu suất, chuẩn **PSR-12** và vượt qua **PHPStan Level 9**.

Trình biên dịch `tphpc` được viết hoàn toàn bằng **Rust thuần (`std` only, không phụ thuộc crate ngoài)**.

---

## Tính năng chính

- **Cú pháp PHP quen thuộc + Định kiểu mạnh**: khai báo kiểu cho biến (`$x: int = 10;`), suy luận kiểu tự động, `const`, `function`, `class`, `interface`, backed `enum`, arrow function `fn (...) => ...`.
- **Generics & Type Aliases**: hỗ trợ generic trên hàm, class, type alias (`identity<T>`, `Box<T>`, `Pair<A, B>`) kèm suy luận tham số kiểu.
- **Structural Shapes**: `type UserShape = { id: int, name: string, email?: string };` kiểm tra key/kiểu lúc compile và biên dịch thành `array` PHP với zero runtime overhead.
- **Union, Nullable & Control-Flow Narrowing**: `int|string|null`, `?string`, tự động thu hẹp kiểu trong nhánh `if`, `else` và sau `return` sớm (`$x === null`, `is_int($x)`, `!is_string($x)`, `$x instanceof Foo`).
- **Hệ thống module đa file**: `import { ... } from './...'`, `namespace`, `use` với cơ chế duyệt đồ thị phụ thuộc, tự động loại bỏ `use` không dùng (tree-shaking) và bỏ qua `require_once` cho `import` chỉ chứa `type` alias.
- **PHPStan Level 9 Gate**: tự động sinh PHPDoc (`@var`, `@param`, `@return`, `@template`) và chạy `phpstan analyse --level=9` ngay trong bước biên dịch.
- **Tuỳ chọn đóng gói & tối ưu**: `--target <8.1|8.2|8.3|8.4>`, `--mode <preserve|bundle>`, `-o/--out <path>`, và `--minify` (nén PHP, rút gọn tên biến/hàm, nén HTML/CSS/JS nhúng trong chuỗi).

---

## Ví dụ nhanh

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

$user: UserShape = ['id' => 1, 'name' => 'Long'];
echo formatId($user['id']) . ' - ' . $user['name'];
```

---

## Cài đặt & Sử dụng CLI

### Biên dịch & Kiểm thử

```bash
# Chạy toàn bộ test (718 test cases trên 7 module)
cargo test

# Kiểm tra kiểu tĩnh (không sinh file .php)
cargo run -- examples/blog_app/public/index.tphp --check

# Biên dịch giữ nguyên cấu trúc thư mục + chạy PHPStan Level 9
cargo run -- examples/blog_app/public/index.tphp

# Biên dịch xuất ra thư mục riêng với target PHP 8.3
cargo run -- examples/blog_app/public/index.tphp --target 8.3 --mode preserve -o dist

# Biên dịch gộp 1 file + minify (PHP, biến/hàm, HTML/CSS/JS)
cargo run -- examples/blog_app/public/index.tphp --bundle --minify -o dist/blog.min.php
```

---

## Cấu trúc thư mục

```text
typephp/
├── src/                    # Source compiler TypePHP viết bằng Rust (std only)
├── tests/                  # 7 bộ test theo module đạt chuẩn MC/DC (718 test cases)
│   ├── lexer_tests.rs
│   ├── parser_tests.rs
│   ├── ast_tests.rs
│   ├── types_checker_tests.rs
│   ├── codegen_tests.rs
│   ├── minify_tests.rs
│   └── e2e_tests.rs
├── docs/                   # Tài liệu ngôn ngữ, kiến trúc compiler, CLI, micro-fw
└── examples/
    ├── demo.tphp           # Demo tổng hợp cú pháp TypePHP
    ├── rest_api/           # REST API chuẩn REST kết nối SQLite
    ├── psr7_microfw/       # Composer package PSR-4 micro-framework (PSR-7 + Router + Validator)
    └── blog_app/           # Ứng dụng Blog + Admin (Tailwind CSS, SQLite, Cookie Login)
```

---

## Tài liệu tham khảo

- [Tài liệu chi tiết (`docs/README.md`)](docs/README.md)
- [Đặc tả ngôn ngữ (`SPEC.md`)](SPEC.md)
- [Hướng dẫn phát triển (`AGENTS.md`)](AGENTS.md)
