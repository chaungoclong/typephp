# Tài liệu TypePHP (`tphp`)

> 🇻🇳 Tiếng Việt | [🇺🇸 English](en/README.md)

**TypePHP** là ngôn ngữ lập trình kiểu tĩnh mạnh (lấy cảm hứng từ TypeScript) giữ nguyên phong cách cú pháp **PHP** và biên dịch sang mã **PHP 8.1–8.4** (`declare(strict_types=1);`) an toàn, tối ưu hiệu suất, vượt qua **PHPStan Level 9** và chuẩn **PSR-12**.

## Mục lục

1. [Cú pháp & Hệ thống kiểu (`language-syntax.md`)](language-syntax.md)
2. [Thiết kế ngôn ngữ & Kiến trúc Compiler (`architecture.md`)](architecture.md)
3. [Trình biên dịch `tphpc` & Tuỳ chọn CLI (`compiler-cli.md`)](compiler-cli.md)
4. [PSR-7 Micro-Framework & Validation (`psr7-microfw.md`)](psr7-microfw.md)
5. [Language Server (LSP), VS Code Extension & Tooling (`lsp-and-tooling.md`)](lsp-and-tooling.md)

---

## Bắt đầu nhanh

### 1. Kiểm thử compiler (Rust)

```bash
cargo test
```

### 2. Biên dịch một file hoặc project đa file

```bash
# Biên dịch giữ nguyên cấu trúc thư mục + chạy PHPStan Level 9
cargo run -- examples/blog_app/public/index.tphp

# Biên dịch gộp thành 1 file + tree-shaking + minify (PHP, biến/hàm, HTML/CSS/JS)
cargo run -- examples/blog_app/public/index.tphp --bundle --minify -o dist/app.php
```

### 3. Các ví dụ đi kèm

- [examples/demo.tphp](../examples/demo.tphp) — Tổng hợp generics, shapes, enums, union narrowing, `main()`.
- [examples/tree_shake_demo/](../examples/tree_shake_demo/main.tphp) — Demo tree-shaking bắc cầu 4 tầng (`main -> lib1 -> lib2 -> lib3`).
- [examples/rest_api/](../examples/rest_api/README.md) — REST API chuẩn REST kết nối SQLite.
- [examples/psr7_microfw/](../examples/psr7_microfw/README.md) — Micro-framework chuẩn PSR-7 + Composer PSR-4 + Validator.
- [examples/blog_app/](../examples/blog_app/README.md) — Ứng dụng Blog + Admin (Tailwind CSS, SQLite, Cookie Login).
