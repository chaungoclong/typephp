# Language Server (LSP), VS Code Extension & Tooling

> 🇻🇳 Tiếng Việt | [🇺🇸 English](en/lsp-and-tooling.md)

Tài liệu này giải thích chi tiết cách hoạt động của **TypePHP Language Server (`tphpc --lsp`)**, **VS Code Extension (`editors/vscode-typephp/`)**, cùng các công cụ cài đặt và môi trường phát triển đi kèm.

---

## 1. TypePHP Language Server (`src/lsp.rs`)

LSP server được tích hợp trực tiếp vào binary `tphpc` thông qua cờ `--lsp` (`src/main.rs`) và viết hoàn toàn bằng Rust `std` (zero external crates).

> **Lưu ý**: `src/lsp.rs` chỉ đọc `Lexer`, `Parser` và `check_with_warnings` để phục vụ editor; **không** sinh file `.php` và **không** làm thay đổi hành vi biên dịch hay kết quả code PHP đầu ra.

### 1.1. Giao thức & Vòng đời kết nối

```text
VS Code (extension.js)                  tphpc --lsp (src/lsp.rs)
         │                                       │
         ├── initialize ────────────────────────►│ trả về capabilities (sync, hover, def, refs)
         ├── textDocument/didOpen ──────────────►│ check_with_warnings() ──► publishDiagnostics
         │                                       │ SymbolIndex::build()  ──► lưu chỉ mục AST
         ├── textDocument/didChange ────────────►│ re-check & rebuild SymbolIndex
         ├── textDocument/hover ────────────────►│ find_at(line, col) ──► Markdown signature/value
         ├── textDocument/definition ───────────►│ nhảy tới định nghĩa / refs / file import
         └── textDocument/references ───────────►│ trả về toàn bộ SymbolOcc cùng def_id
```

- Giao tiếp qua `stdin`/`stdout` dùng chuẩn framing `Content-Length: <bytes>\r\n\r\n<json>`.
- Tự cài đặt bộ parser/serializer JSON (`Json`, `parse_json`, `to_string_compact`) trong `src/lsp.rs`.

### 1.2. Lập chỉ mục AST 2 lượt (`SymbolIndex::build`)

1. **Pass 1 — Đăng ký khai báo**:
   - Duyệt `Program.items` để đăng ký `import`, `type`, `interface` (kèm methods), `enum` (kèm cases và giá trị backing), `class` (kèm fields, methods), và `function`.
   - Dùng `find_ident_span_after` để xác định chính xác `Span` của tên symbol (thay vì từ khóa đứng trước như `class`, `function`, `type`, `const`).
2. **Pass 2 — Duyệt thân hàm/khối lệnh & suy luận kiểu cục bộ**:
   - Duyệt `Stmt`, `Expr`, `TypeExpr` với stack scope (`scopes` cho biến/tham số, `var_types` cho kiểu class của biến).
   - Tự suy luận kiểu cho biến từ type annotation (`PDO $db`), biểu thức `new ClassName(...)`, hoặc kiểu trả về của hàm/method (ví dụ `$stmt = $db->prepare(...)` → `PDOStatement`).
   - Ghi nhận tất cả vị trí xuất hiện (`SymbolOcc`) gồm biến, tham số, hàm, class, interface, enum, constant, method call (`$obj->method()`), static access (`Class::CONST`), `new`, `instanceof`.
   - Với `const` và `enum case`, `format_expr_preview` trích xuất giá trị khởi tạo để hiển thị trực tiếp khi hover.

### 1.3. PHP Reflection Catalog (`PhpCatalog` + `OnceLock`)

Để hỗ trợ đầy đủ toàn bộ hàm, class, interface, method và constant gốc của PHP mà không phải hardcode thủ công:

- Lần đầu cần tra cứu built-in, `PhpCatalog::load_via_php_cli()` chạy `php -r` **một lần duy nhất** và cache vào `std::sync::OnceLock<PhpCatalog>`.
- Script PHP dùng:
  - `get_defined_functions()["internal"]` + `ReflectionFunction` → lấy toàn bộ chữ ký hàm PHP.
  - `get_declared_classes()` / `get_declared_interfaces()` + `ReflectionClass` / `ReflectionMethod` (kể cả `getTentativeReturnType()` trên PHP 8.x và giá trị từ `getConstants()`).
  - `get_defined_constants(true)` → lấy giá trị các hằng số toàn cục của PHP (`PHP_EOL`, `JSON_THROW_ON_ERROR`, `PHP_VERSION`, ...).
- `SymbolIndex` chỉ tạo `SymbolDef` khi symbol built-in thực sự được tham chiếu trong file đang mở (`add_builtin_def` với `span = 0,0`).
- Nếu môi trường không có `php` trong `PATH`, server tự lùi về `PhpCatalog::fallback()`.

### 1.4. Hành vi Hover, `Ctrl + Click` (Definition) & References

- **Hover (`textDocument/hover`)**:
  - Hiển thị loại symbol, chữ ký hoặc giá trị (với constant/enum case), dòng định nghĩa (hoặc `PHP built-in / external symbol`), và số lượng references trong file.
- **`Ctrl + Click` / `F12` (`textDocument/definition`)**:
  - Click tại **nơi sử dụng** → nhảy tới vị trí khai báo (`def.span`).
  - Click vào symbol trong **`import { Foo } from "./file.tphp"`** → tìm và mở file `.tphp` đích tại đúng dòng khai báo `Foo`.
  - Click ngay tại **nơi khai báo** hoặc vào **PHP built-in** → trả về danh sách `Location[]` các vị trí sử dụng (`references`) để VS Code mở Peek References hoặc nhảy tới nơi gọi.
- **Find All References (`textDocument/references`)**:
  - Trả về toàn bộ `SymbolOcc` có cùng `def_id` trong file.

---

## 2. VS Code Extension (`editors/vscode-typephp/`)

Extension chạy thuần Node.js (không cần `npm install` hay build step):

- [package.json](../editors/vscode-typephp/package.json):
  - Khai báo ngôn ngữ `typephp` cho phần mở rộng `.tphp`.
  - Kích hoạt khi mở file `.tphp` (`onLanguage:typephp`).
  - Cấu hình `typephp.serverPath` (mặc định `"tphpc"`).
- [extension.js](../editors/vscode-typephp/extension.js):
  - Lớp `LspClient` spawn `serverPath --lsp` qua `child_process.spawn`.
  - Đăng ký `HoverProvider`, `DefinitionProvider`, `ReferenceProvider` và `DiagnosticCollection` cho ngôn ngữ `typephp`.
- [syntaxes/typephp.tmLanguage.json](../editors/vscode-typephp/syntaxes/typephp.tmLanguage.json): tô màu cú pháp cho từ khóa PHP + TypePHP (`type`, `import`, `from`, `enum`, `case`, shape types, generics).

### Cài đặt nhanh

```bash
# Build release tphpc
cargo build --release

# Symlink extension vào VS Code
ln -s /home/long/Dev/typephp/editors/vscode-typephp ~/.vscode/extensions/typephp
```

Trong VS Code Settings (`settings.json`), trỏ `typephp.serverPath` tới binary nếu chưa nằm trong `PATH`:

```json
{
  "typephp.serverPath": "/home/long/Dev/typephp/target/release/tphpc"
}
```

---

## 3. Công cụ cài đặt & Docker Dev Environment

- **`install/install.sh`**:
  - Tự nhận diện OS/arch (`linux`, `macos`, `windows` qua Git Bash/WSL).
  - Tải binary release hoặc tự cài Rust & biên dịch từ source nếu chưa có binary sẵn, đồng thời cài PHPStan Level 9 vào `~/.typephp`.
- **`Makefile`, `Dockerfile`, `compose.yaml`**:
  - Đóng gói sẵn môi trường Rust + PHP 8.4 + Composer + PHPStan cho contributor không muốn cài công cụ trực tiếp lên máy host.
  - Xem các lệnh dựng/chạy trong `Makefile`.
