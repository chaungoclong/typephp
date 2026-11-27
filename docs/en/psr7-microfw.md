# PSR-7 Micro-Framework & Validation

> [🇻🇳 Tiếng Việt](../psr7-microfw.md) | 🇺🇸 English

The [examples/psr7_microfw/](../../examples/psr7_microfw/) directory provides a reusable micro-framework structured as a **Composer PSR-4 package** (`TypePhp\MicroFw\` → `src/`).

---

## 1. PSR-7 Components (`src/Http/Message/`)

- **Interfaces**:
  - `StreamInterface`, `UriInterface`, `MessageInterface`
  - `RequestInterface extends MessageInterface`
  - `ServerRequestInterface extends RequestInterface`
  - `ResponseInterface extends MessageInterface`
- **Immutable Implementations**:
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

Supports `get()`, `post()`, `put()`, `delete()`, `addMiddleware()`, `handle()`, and `emit()`.

---

## 3. Validation Library (`src/Validation/`)

`Validator::validate(mixed $input, dict<string, string> $rules): ValidationResult`

Supported rules:
- `required` — must be present and non-empty
- `string`, `int`, `bool` — type validation
- `min:<n>`, `max:<n>` — trimmed string length bounds

```php
$res: ValidationResult = $validator->validate($req->getParsedBody(), [
    'title' => 'required|string|min:3|max:150',
    'published' => 'bool',
]);

if (!$res->isValid()) {
    return jsonResponse(422, ['ok' => false, 'errors' => $res->getErrors()]);
}
```
