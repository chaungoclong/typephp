# `typephp/psr7-microfw` (Composer Package + PSR-4)

Micro-framework chuẩn **PSR-7** tổ chức theo cấu trúc **Composer package** & **PSR-4** (`TypePhp\MicroFw\` -> `src/`), mỗi class/interface nằm trong một file riêng tuân thủ chuẩn đặt tên PSR:

```text
examples/psr7_microfw/
├── composer.json
├── public/
│   └── index.tphp
└── src/
    ├── Http/Message/
    │   ├── StreamInterface.tphp
    │   ├── UriInterface.tphp
    │   ├── MessageInterface.tphp
    │   ├── RequestInterface.tphp
    │   ├── ServerRequestInterface.tphp
    │   ├── ResponseInterface.tphp
    │   ├── StringStream.tphp
    │   ├── Uri.tphp
    │   ├── ServerRequest.tphp
    │   └── Response.tphp
    └── Routing/
        └── App.tphp
```

## 1. Biên dịch toàn bộ package từ `.tphp` sang `.php`

```bash
cargo run -- examples/psr7_microfw/public/index.tphp
```

## 2. Khởi chạy server PHP

```bash
php -S 127.0.0.1:8081 examples/psr7_microfw/public/index.php
```

## 3. Kiểm thử bằng `curl`

```bash
curl -s http://127.0.0.1:8081/
curl -s "http://127.0.0.1:8081/hello?name=TypePHP"
curl -s -X POST http://127.0.0.1:8081/echo \
  -H "Content-Type: application/json" \
  -d '{"hello":"psr4"}'
```
