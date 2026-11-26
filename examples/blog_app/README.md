# TypePHP Blog App (UI Tailwind CSS + Admin + SQLite)

Ứng dụng Blog độc lập nằm ngoài source của framework `examples/psr7_microfw`, sử dụng **PSR-7 Micro-Framework**, **Validator**, **SQLite** và **Tailwind CSS** với 2 trang giao diện:
- **Blog**: `GET /` (danh sách bài viết đã xuất bản) và `GET /post?id={id}` (chi tiết bài viết)
- **Admin**: `GET /admin` (giao diện quản trị tạo/sửa/xoá bài viết, xác thực bằng `X-Admin-Token: secret-admin-token`)

## 1. Biên dịch

```bash
cargo run -- examples/blog_app/public/index.tphp
```

## 2. Khởi chạy server PHP

```bash
php -S 127.0.0.1:8082 examples/blog_app/public/index.php
```

Mở trình duyệt:
- Trang Blog: `http://127.0.0.1:8082/`
- Trang Admin: `http://127.0.0.1:8082/admin`
