# Thiết kế Ngôn ngữ & Kiến trúc Compiler (`tphpc`)

> 🇻🇳 Tiếng Việt | [🇺🇸 English](en/architecture.md)

Tài liệu này mô tả triết lý thiết kế ngôn ngữ **TypePHP** và cơ chế hoạt động chi tiết của từng module trong trình biên dịch `tphpc` (viết bằng Rust thuần, không phụ thuộc crate ngoài).

---

## 1. Triết lý thiết kế ngôn ngữ

1. **Giữ nguyên cú pháp PHP**: Không thay đổi thói quen viết code PHP (`$var`, `function`, `->`, `::`, `.`, `foreach`, `elseif`, `fn (...) => ...`, `['k' => v]`, `namespace`, `use`), giúp developer PHP dùng được ngay và tái sử dụng toàn bộ hệ sinh thái PHP (PDO, PSR-7, Composer).
2. **Định kiểu tĩnh mạnh hơn PHP thuần**:
   - **Biến cục bộ có kiểu**: `$count: int = 0;`, `$items: int[] = [1, 2, 3];`, `$map: dict<string, int> = [];` hoặc tự suy luận kiểu.
   - **Type Alias & Structural Shape**: `type UserShape = { id: int, name: string, email?: string };` kiểm tra key và kiểu dữ liệu lúc biên dịch.
   - **Generics**: `function identity<T>(T $x): T` và `class Box<T>` có suy luận tham số kiểu (`infer_type_params`).
   - **Control-Flow Narrowing**: tự động thu hẹp kiểu union trong nhánh `if` và sau `return` sớm.
3. **An toàn, PSR-12 & Zero Runtime Overhead**:
   - Mọi `type` alias, shape và generic parameter được kiểm tra ở compile-time rồi xoá bỏ khi sinh PHP.
   - Mã PHP sinh ra tuân thủ `declare(strict_types=1);`, chuẩn **PSR-12**, và kèm PHPDoc chi tiết để vượt qua **PHPStan Level 9**.

---

## 2. Pipeline biên dịch

```text
Source (.tphp)
   │
   ▼  load_all_modules() (DFS topo theo import/require_once)
[1] Lexer        (src/lexer.rs)   ──► Vec<Token>
   │
   ▼
[2] Parser       (src/parser.rs)  ──► Program (AST trong src/ast.rs)
   │
   ▼
[3] TypeChecker  (src/checker.rs) ──► Kiểm tra kiểu, generics, narrowing (src/types.rs)
   │
   ▼
[4] Codegen      (src/codegen.rs) ──► Mã PHP 8.x chuẩn PSR-12 + PHPDoc + Tree-shake `use`
   │
   ▼
[5] PHPStan L9   (src/lib.rs)     ──► Cổng kiểm tra bắt buộc (level=9)
   │
   ▼  (Nếu bật --minify)
[6] Minifier     (src/minify.rs + minify_php) ──► Mangle biến/hàm + nén PHP/HTML/CSS/JS
```

---

## 3. Chi tiết từng module trong `src/`

### `src/error.rs` — Biểu diễn lỗi & vị trí nguồn
- **`Span { line, col }`**: lưu dòng và cột của từng token / node AST.
- **`CompileError { stage, message, span }`**: phân loại lỗi rõ ràng theo từng giai đoạn: `LexError`, `ParseError`, `TypeError`.

### `src/lexer.rs` — Bộ phân tích từ vựng (`Lexer`)
- Chuyển chuỗi nguồn `.tphp` thành `Vec<Token>`.
- Tự động bỏ qua thẻ mở/đóng `<?php` và `?>`, khoảng trắng, comment `//`, `#`, `/* ... */`.
- Nhận diện biến PHP (`TokenKind::Var` bắt đầu bằng `$`), identifier/keyword (`function`, `fn`, `class`, `interface`, `enum`, `case`, `type`, `import`, `from`, `namespace`, `use`, `echo`, ...), toán tử (`->`, `::`, `=>`, `??`, `===`, `!==`, `\`).

### `src/ast.rs` — Cây cú pháp trừu tượng (AST)
- **`Program`**: danh sách các `Item` (`Namespace`, `Use`, `Require`, `Import`, `TypeAlias`, `Interface`, `Enum`, `Class`, `Function`, `Stmt`).
- **`TypeExpr`**: biểu diễn cú pháp kiểu (`Named`, `Array`, `Union`, `Optional`, `Shape`, `Function`).
- **`Stmt` & `Expr`**: biểu diễn câu lệnh (`VarDecl`, `ConstDecl`, `Assign`, `Return`, `Echo`, `If`, `While`, `ForEach`, `ExprStmt`) và biểu thức (`Binary`, `Unary`, `Cast`, `Call`, `New`, `Member`, `StaticAccess`, `Index`, `ArrowFn`, `InstanceOf`, `ArrayLit`, `AssocArrayLit`).

### `src/parser.rs` — Bộ phân tích cú pháp (`Parser`)
- Parser đệ quy (recursive descent) kết hợp phân tầng độ ưu tiên toán tử (`??` → `||` → `&&` → `===`/`!==` → `<`/`<=`/`>`/`>=`/`instanceof` → `+`/`-`/`.` → `*`/`/`/`%` → unary `-`/`!`/cast `(int)` → postfix `()`/`->`/`::`/`[]`).
- Phân biệt mảng tuần tự `[1, 2]` và mảng kết hợp `['k' => v]` dựa vào token `=>` sau phần tử đầu.
- Hỗ trợ generic call `identity<int>(42)` và `new Box<int>(99)` bằng lookahead `is_generic_call_ahead`.

### `src/types.rs` — Mô hình hệ thống kiểu (`Type`)
- Định nghĩa kiểu ngữ nghĩa: `Int`, `Float`, `String`, `Bool`, `Void`, `Never`, `Null`, `Mixed`, `Any`, `Array(Box<Type>)`, `Dict(Box<Type>, Box<Type>)`, `Union(Vec<Type>)`, `Shape(...)`, `Function { params, ret }`, `Class(String, Vec<Type>)`, `Enum(String)`, `TypeParam(String)`.
- **`Type::union`**: làm phẳng và loại trùng lặp các thành viên union.
- **`Type::substitute`**: thế các `TypeParam` bằng kiểu cụ thể khi khởi tạo generic.
- **`Type::is_assignable_to`**: kiểm tra tính tương thích kiểu (primitive, union, covariant array/dict, structural shape checking với field optional `?`, contravariant/covariant function type).

### `src/checker.rs` — Bộ kiểm tra kiểu tĩnh (`TypeChecker`)
Kiểm tra theo **3 lượt (passes)** trên mỗi `Program`:
1. **Pass 0**: Đăng ký trước tên `Interface` và `Enum` để hỗ trợ tham chiếu chéo (ví dụ `MessageInterface` tham chiếu `StreamInterface`).
2. **Pass 1**: Đăng ký `TypeAlias`, giải quyết chữ ký method của `Interface` (bao gồm kế thừa `extends`), đăng ký `Class` (fields, constructor, methods) và `Function` top-level.
3. **Pass 2**: Kiểm tra thân `Class` (đối chiếu `implements` với interface), thân `Function` và các `Stmt` top-level.

Các cơ chế nổi bật:
- **`is_assignable` & `is_subclass_or_implements`**: kiểm tra gán class cho interface hoặc class cha.
- **`check_generic_call` & `infer_type_params`**: thế tham số kiểu tường minh hoặc tự suy luận từ đối số truyền vào.
- **`extract_narrowing` & `always_returns`**: thu hẹp kiểu union trong `if` / `else` và sau các nhánh `if (...) { return ...; }`, hỗ trợ cả phủ định `!` (ví dụ `!is_string($x)`).
- **`register_builtins`**: khai báo sẵn kiểu cho superglobals (`$_SERVER`, `$_GET`, `$_COOKIE`), hằng (`PHP_URL_PATH`, `__DIR__`), class (`PDO`, `PDOStatement`) và các hàm PHP thông dụng (`json_encode`, `json_decode`, `trim`, `substr`, `str_starts_with`, `htmlspecialchars`, `setcookie`, ...).

### `src/codegen.rs` — Bộ sinh mã PHP (`Codegen`)
- **`emit_program_with_aliases`** (`--mode preserve`): sinh từng file `.php` với `declare(strict_types=1);`, chuẩn **PSR-12** (Allman braces cho class/interface/enum/function/method, khoảng trắng sau `fn`, single trailing newline).
- **`emit_bundle`** (`--mode bundle`): gộp nhiều module thành 1 file `.php`, tự động bọc các `namespace` bằng cú pháp `namespace ... { ... }`, bỏ qua `require_once` nội bộ `.tphp`, và chạy **`compute_reachable_symbols`** (BFS tree-shaking từ `main()` / top-level `Stmt`) để loại bỏ toàn bộ khai báo không được sử dụng.
- **`entry_auto_main_call`**: tự động phát sinh `main();` hoặc `exit(main());` (hỗ trợ cả `main(string[] $args)` và `main(int $argc, string[] $argv)`) ở cuối file entrypoint khi có định nghĩa `function main(...)`.
- **`format_phpstan_type` & `emit_fn_phpdoc`**: tự động sinh `@var`, `@param`, `@return`, `@template` (`list<T>`, `array<K, V>`, `array{...}`, `\Closure(...): ...`) để đáp ứng PHPStan Level 9.
- **Tối ưu import & dead-import elimination**:
  - `collect_used_symbols`: quét toàn bộ AST để loại bỏ các câu lệnh `use ...;` không được sử dụng.
  - Khử trùng lặp `require_once` (`seen_requires`) và bỏ qua `require_once` cho các `import` chỉ dùng `type` alias.
- **`minify_php`**: nén khoảng trắng và xoá comment/PHPDoc ở tầng token PHP sau khi đã kiểm tra PHPStan.

### `src/minify.rs` — Rút gọn tên biến/hàm & nén Web nội tuyến
- **`mangle_programs`**:
  - Đổi tên biến cục bộ và tham số thành `$a`, `$b`, ... (`VarMangler`), giữ nguyên `$this`, superglobals (`$_GET`, `$_SERVER`, `$_COOKIE`, ...) và tên property (`$this->prop`).
  - Ở chế độ `bundle`, đổi tên hàm top-level người dùng định nghĩa thành `_a`, `_b`, ... theo từng namespace.
- **`minify_html_css_js`**: tự động nhận diện chuỗi chứa HTML (`<!DOCTYPE`, `<div`, `<script>`, ...) để nén HTML (xoá `<!-- ... -->`, gộp `> <`), CSS trong `<style>` và JS trong `<script>`.

### `src/lib.rs` & `src/main.rs` — Điều phối module, CLI & PHPStan Gate
- **`load_all_modules` / `load_module_dfs`**: duyệt đồ thị phụ thuộc theo thứ tự topo, chống vòng lặp vô hạn bằng `visited: HashSet<PathBuf>`.
- **`common_ancestor`**: tính thư mục gốc chung để bảo toàn cấu trúc tương đối khi xuất ra thư mục `-o <dir>`.
- **`find_phpstan` & `run_phpstan_level9`**: chạy `phpstan analyse --level=9 --no-progress` lên toàn bộ file `.php` sinh ra; biên dịch chỉ thành công khi PHPStan Level 9 pass hoàn toàn.

### `src/lsp.rs` — Language Server (`tphpc --lsp`)
- Chạy khi truyền `--lsp`; hoàn toàn tách biệt và không ảnh hưởng tới đầu ra `.php` của compiler.
- Xem chi tiết cơ chế `SymbolIndex`, `PhpCatalog` (PHP Reflection), VS Code extension và các công cụ đi kèm tại [lsp-and-tooling.md](lsp-and-tooling.md).
