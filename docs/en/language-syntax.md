# TypePHP Syntax & Type System

> [🇻🇳 Tiếng Việt](../language-syntax.md) | 🇺🇸 English

TypePHP keeps familiar PHP constructs (`$var`, `function`, `->`, `::`, `.`, `foreach`, `elseif`, `fn (...) => ...`, `global`) while adding a static type system checked at compile time.

---

## 1. Variables & Types

### Explicit Type Annotations

```php
$count: int = 10;
$price: float = 19.99;
$name: string = 'TypePHP';
$active: bool = true;
const ADMIN_TOKEN: string = 'secret';
```

### Type Inference

```php
$message = 'Hello'; // inferred as string
$items: int[] = [1, 2, 3];
$scores: dict<string, int> = ['alice' => 10, 'bob' => 20];
```

---

## 2. Advanced Types

- **Union & Nullable**: `int|string|null`, `?string` (equivalent to `string|null`).
- **Structural Shapes**:
  ```php
  type UserShape = {
      id: int,
      name: string,
      email?: string,
  };

  $u: UserShape = ['id' => 1, 'name' => 'Long'];
  ```
- **Function Types**:
  ```php
  type Handler = (ServerRequestInterface) => ResponseInterface;
  ```
- **Generics** (functions & classes):
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

## 3. OOP, Interfaces & Backed Enums

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

The `TypeChecker` narrows union types inside `if` branches and after early `return` statements:

```php
function formatId(int|string|null $id): string {
    if ($id === null) {
        return 'none';
    }
    if (is_int($id)) {
        return 'ID#' . (string) $id;
    }
    return $id; // $id is narrowed to string
}
```

Supported narrowing checks: `$x === null`, `$x !== null`, `is_int($x)`, `is_string($x)`, `is_bool($x)`, `is_float($x)`, `$x instanceof Foo`, and negations `!` (e.g. `if (!is_string($x)) { return ...; }`).

---

## 5. Modules: `namespace`, `use` and `import`

```php
<?php

namespace TypePhp\BlogApp\Repository;

import { ServerRequestInterface } from '../../psr7_microfw/src/Http/Message/ServerRequestInterface.tphp';
use TypePhp\MicroFw\Http\Message\ServerRequestInterface;
```

---

## 6. Superglobals, `global` and `main()` Entrypoint

- **PHP Superglobals**: `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `$_SESSION`, `$_FILES`, `$_REQUEST`, `$_ENV`, and `$GLOBALS` are pre-declared in the type checker.
- **`global` keyword**: `global $db, $config;` is supported inside functions/methods and binds global-scope variables into the local scope.
- **Optional Go/C-style `main()`**:
  ```php
  function main(): void {
      echo "Hello TypePHP\n";
  }
  // Or return an exit code: function main(): int { return 0; }
  // Or accept CLI arguments: function main(string[] $args): int { ... }
  ```
  The compiler automatically emits `main();` (or `exit(main());`) at the end of the entrypoint if `main()` is not already called at top-level.
