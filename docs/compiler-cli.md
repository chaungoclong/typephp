# Trình biên dịch `tphpc` & Tuỳ chọn CLI

> 🇻🇳 Tiếng Việt | [🇺🇸 English](en/compiler-cli.md)

Trình biên dịch `tphpc` được viết bằng Rust thuần (`std` only) gồm các giai đoạn:
**Lexer → Parser → TypeChecker → Codegen (PSR-12 + PHPDoc + Tree-Shaking) → PHPStan Level 9 Gate → (tuỳ chọn) Minifier**.

---

## Cú pháp dòng lệnh

```bash
tphpc <input.tphp> [OPTIONS]
```

| Cờ (Option) | Mô tả | Mặc định |
| :--- | :--- | :--- |
| `-o, --out <path>` | Đường dẫn file output (khi `--mode bundle`) hoặc thư mục gốc output (khi `--mode preserve`) | Cùng thư mục file `.tphp` |
| `--target <8.1\|8.2\|8.3\|8.4>` | Chọn phiên bản PHP mục tiêu | `8.4` |
| `--mode <preserve\|bundle>` | `preserve`: giữ nguyên cấu trúc thư mục; `bundle`: gộp toàn bộ thành 1 file `.php` + tree-shaking | `preserve` |
| `--bundle` | Viết tắt của `--mode bundle` | `false` |
| `--minify` | Nén mã PHP, rút gọn tên biến/hàm, nén HTML/CSS/JS nhúng trong chuỗi | `false` |
| `--check` | Chỉ kiểm tra kiểu tĩnh, không sinh file `.php` | `false` |
| `--lsp` | Khởi chạy TypePHP Language Server (LSP qua stdio) cho IDE/editor | `false` |

---

## Ví dụ sử dụng

### 1. Biên dịch giữ nguyên cấu trúc vào thư mục `dist/`

```bash
cargo run -- examples/blog_app/public/index.tphp --mode preserve -o dist
```

### 2. Biên dịch gộp 1 file + Target PHP 8.2 + Minify

```bash
cargo run -- examples/blog_app/public/index.tphp --target 8.2 --bundle --minify -o dist/blog.min.php
```

---

## Cổng kiểm tra PHPStan Level 9

Mọi lần biên dịch file (không phải `--check`) đều tự động chạy:

```bash
phpstan analyse --level=9 --no-progress <files...>
```

- Nếu PHPStan Level 9 phát hiện lỗi, `tphpc` sẽ dừng và trả exit code `1`.
- Khi bật `--minify`, `tphpc` kiểm tra PHPStan Level 9 trên bản mã đầy đủ PHPDoc **trước**, sau đó mới thực hiện mangle tên biến/hàm và nén code.
