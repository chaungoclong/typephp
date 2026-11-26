# TypePHP REST API Example (SQLite)

REST API quản lý `tasks` viết bằng **TypePHP** ([index.tphp](index.tphp)), biên dịch sang PHP 8.4 ([index.php](index.php)) và lưu trữ dữ liệu bằng SQLite (`PDO`).

## 1. Biên dịch từ `.tphp` sang `.php`

Từ thư mục gốc của project:

```bash
cargo run -- examples/rest_api/index.tphp -o examples/rest_api/index.php
```

## 2. Khởi chạy server PHP

```bash
php -S 127.0.0.1:8080 examples/rest_api/index.php
```

## 3. Kiểm thử bằng `curl`

### Tạo task mới (`POST /tasks`)

```bash
curl -i -X POST http://127.0.0.1:8080/tasks \
  -H "Content-Type: application/json" \
  -d '{"title":"Build TypePHP"}'
```

### Lấy danh sách tasks (`GET /tasks`)

```bash
curl -s http://127.0.0.1:8080/tasks
```

### Lấy chi tiết 1 task (`GET /tasks/{id}`)

```bash
curl -s http://127.0.0.1:8080/tasks/1
```

### Cập nhật task (`PUT /tasks/{id}`)

```bash
curl -s -X PUT http://127.0.0.1:8080/tasks/1 \
  -H "Content-Type: application/json" \
  -d '{"title":"Build TypePHP Compiler","completed":1}'
```

### Xoá task (`DELETE /tasks/{id}`)

```bash
curl -s -X DELETE http://127.0.0.1:8080/tasks/1
```
