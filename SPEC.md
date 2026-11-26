# TypePHP (`tphp`) Language Specification

TypePHP is a statically typed superset of **PHP** (inspired by TypeScript's type system) that compiles to clean, idiomatic **PHP 8.4** (`declare(strict_types=1);`).

## Design Goals

1. **Familiar PHP syntax**: Keep `$variables`, `function`, `->` member access, `::` static/enum access, `.` string concatenation, `['key' => value]` associative arrays, `foreach ($xs as $x)`, `elseif`, `fn(...) => ...`, and `case` in `enum`.
2. **Stronger static type system**:
   - **Typed local variables**: `$count: int = 0;`, `$names: string[] = ["Alice", "Bob"];`
   - **Generics**: `function identity<T>(T $x): T`, `class Box<T> { private T $value; ... }`, `new Box<int>(42)`
   - **Type aliases & structural array shapes**: `type UserShape = { id: int, name: string, email?: string };`
   - **Union & nullable types**: `int|string|null`, `?string`
   - **Function types**: `(int, string) => bool`
   - **Control-flow type narrowing**: `$x !== null`, `is_int($x)`, `is_string($x)`, `$x instanceof Foo`
3. **Zero runtime overhead**: Type aliases, variable type annotations, structural shapes, and generic type parameters are checked at compile time and erased (with optional PHPDoc) in the emitted PHP 8.4 output.

---

## 1. Variables & Expressions (PHP Style)

```php
<?php

$count: int = 0;
$name = "TypePHP"; // inferred as string
const MAX_RETRIES: int = 3;

$greeting: string = "Hello, " . $name . "!";
$scores: dict<string, int> = ["alice" => 10, "bob" => 20];
```

---

## 2. Type System

- **Primitives**: `int`, `float`, `string`, `bool`, `void`, `never`, `null`, `mixed`, `any`
- **Arrays & Maps**:
  - `int[]` or `array<int>` (homogeneous list -> `array` in PHP)
  - `dict<string, int>` (associative array -> `array` in PHP)
- **Union & Nullable**: `int|string`, `?string` (desugars to `string|null`)
- **Structural Shapes**:
  ```php
  type UserShape = {
      id: int,
      name: string,
      email?: string,
  };

  $u: UserShape = ['id' => 1, 'name' => 'Long'];
  ```
- **Generics**:
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

  $box = new Box<int>(42);
  $n: int = $box->get();
  ```

---

## 3. Functions, Classes, Interfaces, Enums

```php
interface Greeter {
    public function greet(): string;
}

enum Status: string {
    case Active = 'active';
    case Inactive = 'inactive';
}

class Person implements Greeter {
    public string $name;
    private int $age;

    public function __construct(string $name, int $age) {
        $this->name = $name;
        $this->age = $age;
    }

    public function greet(): string {
        return "Hello, I am " . $this->name;
    }
}

$double = fn(int $x): int => $x * 2;
```

---

## 4. Control Flow & Type Narrowing

```php
function formatId(int|string|null $id): string {
    if ($id === null) {
        return "none";
    }
    if (is_int($id)) {
        // $id is narrowed to int
        return "ID#" . (string)$id;
    }
    // $id is narrowed to string
    return $id;
}
```
