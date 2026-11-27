# PSR-7 Micro-Framework & Validation

> 🇻🇳 Tiếng Việt | [🇺🇸 English](en/psr7-microfw.md)

Thư mục [examples/psr7_microfw/](../examples/psr7_microfw/) cung cấp một micro-framework tổ chức theo chuẩn **Composer package + PSR-4** (`TypePhp\MicroFw\` → `src/`).

---

## 1. Thành phần PSR-7 (`src/Http/Message/`)

- **Interfaces**:
  - `StreamInterface`, `UriInterface`, `MessageInterface`
  - `RequestInterface extends MessageInterface`
  - `ServerRequestInterface extends RequestInterface`
  - `ResponseInterface extends MessageInterface`
- **Implementations bất biến**:
  - `StringStream`, `Uri`, `ServerRequest` (`withHeader`, `withBody`, `withAttribute`), `Response` (`withHeader`, `withBody`, `withStatus`).

---

## 2. Router & Middleware (`src/Routing/App.tphp`)

```php
$app = new App();

$app->addMiddleware(fn(ServerRequestInterface $req, Handler $next): ResponseInterface =>
    $next($req->withAttribute('app', 'TypePHP'))
);

$app->get('/', fn(ServerRequestInterface $req): ResponseInterface =>
    new Response(200, ['content-type' => 'text/plain'], new StringStream('OK'))
);
```

Hỗ trợ `get()`, `post()`, `put()`, `delete()`, `addMiddleware()`, `handle()`, `emit()`.

---

## 3. Thư viện Validation (`src/Validation/`)

`Validator::validate(mixed $input, dict<string, string> $rules): ValidationResult`

Các rule hỗ trợ:
- `required` — bắt buộc có mặt và không rỗng
- `string`, `int`, `bool` — kiểm tra đúng kiểu dữ liệu
- `min:<n>`, `max:<n>` — kiểm tra độ dài chuỗi sau khi `trim`

```php
$res: ValidationResult = $validator->validate($req->getParsedBody(), [
    'title' => 'required|string|min:3|max:150',
    'published' => 'bool',
]);

if (!$res->isValid()) {
    return jsonResponse(422, ['ok' => false, 'errors' => $res->getErrors()]);
}
```
