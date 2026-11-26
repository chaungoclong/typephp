# Cú pháp & Hệ thống kiểu TypePHP

TypePHP giữ nguyên các thành phần quen thuộc của PHP (`$var`, `function`, `->`, `::`, `.`, `foreach`, `elseif`, `fn (...) => ...`) và bổ sung hệ thống kiểu tĩnh kiểm tra lúc biên dịch.

---

## 1. Biến & Kiểu dữ liệu

### Khai báo biến có kiểu tường minh

```php
$count: int = 10;
$price: float = 19.99;
$name: string = 'TypePHP';
$active: bool = true;
const ADMIN_TOKEN: string = 'secret';
```

### Suy luận kiểu (Type Inference)

```php
$message = 'Hello'; // tự suy luận là string
$items: int[] = [1, 2, 3];
$scores: dict<string, int> = ['alice' => 10, 'bob' => 20];
```

---

## 2. Kiểu nâng cao

- **Union & Nullable**: `int|string|null`, `?string` (tương đương `string|null`).
- **Structural Shape (Mảng kết cấu)**:
  ```php
  type UserShape = {
      id: int,
      name: string,
      email?: string,
  };

  $u: UserShape = ['id' => 1, 'name' => 'Long'];
  ```
- **Function Type**:
  ```php
  type Handler = (ServerRequestInterface) => ResponseInterface;
  ```
- **Generics** (hàm & class):
  ```php
  function identity<T>(T $value): T {
      return $value;
  }

  class Box<T> {
      private T $value;
      public function __construct(T $value) {
          $this->value = $value;
      }
      public function get(): T {
          return $this->value;
      }
  }
  ```

---

## 3. OOP, Interface & Backed Enum

```php
interface Greeter {
    public function greet(): string;
}

enum Status: string {
    case Active = 'active';
    case Inactive = 'inactive';
}

class User implements Greeter {
    public string $name;
    public function __construct(string $name) {
        $this->name = $name;
    }
    public function greet(): string {
        return 'Hello, ' . $this->name;
    }
}
```

---

## 4. Control-Flow Type Narrowing

TypeChecker tự động thu hẹp kiểu union trong các nhánh `if` và sau lệnh `return` sớm:

```php
function formatId(int|string|null $id): string {
    if ($id === null) {
        return 'none';
    }
    if (is_int($id)) {
        return 'ID#' . (string) $id;
    }
    return $id; // $id được thu hẹp thành string
}
```

Hỗ trợ narrowing với `$x === null`, `$x !== null`, `is_int($x)`, `is_string($x)`, `is_bool($x)`, `is_float($x)`, `$x instanceof Foo`, và phủ định `!` (ví dụ `if (!is_string($x)) { return ...; }`).

---

## 5. Module: `namespace`, `use` và `import`

```php
<?php

namespace TypePhp\BlogApp\Repository;

import { ServerRequestInterface } from '../../psr7_microfw/src/Http/Message/ServerRequestInterface.tphp';
use TypePhp\MicroFw\Http\Message\ServerRequestInterface;
```

- `import`: compiler tự động duyệt cây phụ thuộc `.tphp`, kiểm tra kiểu chéo giữa các file và biên dịch toàn bộ module liên quan.
- `use` không sử dụng sẽ tự động bị loại bỏ khi sinh mã PHP.
- `import` chỉ dùng để lấy `type` alias sẽ bị bỏ qua `require_once` ở runtime.
